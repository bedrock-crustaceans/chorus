use crate::level::generator::legacy::beta_173::java_rand::JavaRand;

#[derive(Clone, Debug)]
pub struct PerlinNoise {
    perm: [u8; 512],
}

impl PerlinNoise {
    pub fn new(rng: &mut JavaRand) -> Self {
        let mut p = [0u8; 256];

        for i in 0..256 {
            p[i] = i as u8;
        }

        for i in (0..256).rev() {
            let j = rng.next_i32_bounded((i + 1) as i32) as usize;
            p.swap(i, j);
        }

        let mut perm = [0u8; 512];
        for i in 0..512 {
            perm[i] = p[i & 255];
        }

        PerlinNoise { perm }
    }

    fn fade(t: f64) -> f64 {
        t * t * t * (t * (t * 6.0 - 15.0) + 10.0)
    }

    fn lerp(t: f64, a: f64, b: f64) -> f64 {
        a + t * (b - a)
    }

    fn grad(hash: u8, x: f64, y: f64, z: f64) -> f64 {
        let h = (hash & 15) as i32;
        let u = if h < 8 { x } else { y };
        let v = if h < 4 {
            y
        } else if h == 12 || h == 14 {
            x
        } else {
            z
        };

        let u = if (h & 1) == 0 { u } else { -u };
        let v = if (h & 2) == 0 { v } else { -v };

        u + v
    }

    pub fn noise_3d(&self, x: f64, y: f64, z: f64) -> f64 {
        let xi = x.floor() as i32 & 255;
        let yi = y.floor() as i32 & 255;
        let zi = z.floor() as i32 & 255;

        let xf = x - x.floor();
        let yf = y - y.floor();
        let zf = z - z.floor();

        let u = Self::fade(xf);
        let v = Self::fade(yf);
        let w = Self::fade(zf);

        let a  = self.perm[xi as usize] as i32 + yi;
        let aa = self.perm[a as usize] as i32 + zi;
        let ab = self.perm[(a + 1) as usize] as i32 + zi;
        let b  = self.perm[(xi + 1) as usize] as i32 + yi;
        let ba = self.perm[b as usize] as i32 + zi;
        let bb = self.perm[(b + 1) as usize] as i32 + zi;

        let aa0 = self.perm[aa as usize];
        let ab0 = self.perm[ab as usize];
        let ba0 = self.perm[ba as usize];
        let bb0 = self.perm[bb as usize];

        let x1 = Self::lerp(
            u,
            Self::grad(aa0, xf,     yf,     zf),
            Self::grad(ba0, xf - 1.0, yf,     zf),
        );
        let x2 = Self::lerp(
            u,
            Self::grad(ab0, xf,     yf - 1.0, zf),
            Self::grad(bb0, xf - 1.0, yf - 1.0, zf),
        );
        let y1 = Self::lerp(v, x1, x2);

        let aa1 = self.perm[(aa + 1) as usize];
        let ab1 = self.perm[(ab + 1) as usize];
        let ba1 = self.perm[(ba + 1) as usize];
        let bb1 = self.perm[(bb + 1) as usize];

        let x3 = Self::lerp(
            u,
            Self::grad(aa1, xf,     yf,     zf - 1.0),
            Self::grad(ba1, xf - 1.0, yf,     zf - 1.0),
        );
        let x4 = Self::lerp(
            u,
            Self::grad(ab1, xf,     yf - 1.0, zf - 1.0),
            Self::grad(bb1, xf - 1.0, yf - 1.0, zf - 1.0),
        );
        let y2 = Self::lerp(v, x3, x4);

        Self::lerp(w, y1, y2)
    }
}