use super::helpers::GRADIENT;
use crate::shared::rng::random::RandomSource;

pub struct SimplexNoise {
    perms: [u8; 256],
}

impl SimplexNoise {
    const F2: f64 = 0.5 * (1.7320508075688772 - 1.0);
    const G2: f64 = (3.0 - 1.7320508075688772) / 6.0;

    pub fn without_offset(random: &mut impl RandomSource) -> Self {
        for _ in 0..3 {
            random.next_double();
        }
        let mut perms = [0u8; 256];
        for (i, perm) in perms.iter_mut().enumerate() {
            *perm = i as u8;
        }
        for i in 0..256 {
            let offset = random.next_int_bounded(256 - i as i32) as usize;
            perms.swap(i, offset + i);
        }
        Self { perms }
    }

    fn permute(&self, x: i32) -> i32 {
        self.perms[(x & 0xFF) as usize] as i32
    }

    fn corner(index: i32, x: f64, y: f64) -> f64 {
        let t = 0.5 - x * x - y * y;
        if t < 0.0 {
            return 0.0;
        }
        let [gx, gy, _] = GRADIENT[index as usize];
        t * t * t * t * (gx as f64 * x + gy as f64 * y)
    }

    pub fn get(&self, x: f64, y: f64) -> f32 {
        let (xin, yin) = (x, y);
        let s = (xin + yin) * Self::F2;
        let i = (xin + s).floor() as i32;
        let j = (yin + s).floor() as i32;
        let t = (i + j) as f64 * Self::G2;
        let x0 = xin - (i as f64 - t);
        let y0 = yin - (j as f64 - t);
        let (i1, j1) = if x0 > y0 { (1, 0) } else { (0, 1) };
        let x1 = x0 - i1 as f64 + Self::G2;
        let y1 = y0 - j1 as f64 + Self::G2;
        let x2 = x0 - 1.0 + 2.0 * Self::G2;
        let y2 = y0 - 1.0 + 2.0 * Self::G2;
        let (ii, jj) = (i & 0xFF, j & 0xFF);
        let gi0 = self.permute(ii + self.permute(jj)) % 12;
        let gi1 = self.permute(ii + i1 + self.permute(jj + j1)) % 12;
        let gi2 = self.permute(ii + 1 + self.permute(jj + 1)) % 12;
        (70.0 * (Self::corner(gi0, x0, y0) + Self::corner(gi1, x1, y1) + Self::corner(gi2, x2, y2))) as f32
    }
}
