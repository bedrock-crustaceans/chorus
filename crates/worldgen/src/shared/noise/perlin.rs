use super::helpers::{GRADIENT, lerp3, smoothstep, wrap};
use crate::shared::rng::random::RandomSource;

const PERLIN_GRADIENT: [[f32; 3]; 16] = {
    let mut out = [[0.0; 3]; 16];
    let mut i = 0;
    while i < 16 {
        out[i] = [GRADIENT[i][0] as f32, GRADIENT[i][1] as f32, GRADIENT[i][2] as f32];
        i += 1;
    }
    out
};

fn grad_dot(hash: i32, x: f32, y: f32, z: f32) -> f32 {
    let [gx, gy, gz] = PERLIN_GRADIENT[(hash & 15) as usize];
    gx * x + gy * y + gz * z
}

pub struct PerlinNoise {
    perms: [u8; 256],
    offset_x: f64,
    offset_y: f64,
    offset_z: f64,
    smear_y: Option<f64>,
}

impl PerlinNoise {
    pub fn new(random: &mut impl RandomSource) -> Self {
        let offset_x = random.next_double() * 256.0;
        let offset_y = random.next_double() * 256.0;
        let offset_z = random.next_double() * 256.0;
        let mut perms = [0u8; 256];
        for (i, perm) in perms.iter_mut().enumerate() {
            *perm = i as u8;
        }
        for i in 0..256 {
            let offset = random.next_int_bounded(256 - i as i32) as usize;
            perms.swap(i, offset + i);
        }
        Self {
            perms,
            offset_x,
            offset_y,
            offset_z,
            smear_y: None,
        }
    }

    fn permute(&self, x: i32) -> i32 {
        self.perms[(x & 0xFF) as usize] as i32
    }

    pub fn get(&self, raw_x: f64, raw_y: f64, raw_z: f64) -> f32 {
        let x = wrap(raw_x) + self.offset_x;
        let y = wrap(raw_y) + self.offset_y;
        let z = wrap(raw_z) + self.offset_z;
        let floor_x = x.floor() as i32;
        let floor_y = y.floor() as i32;
        let floor_z = z.floor() as i32;
        let relative_x = (x - floor_x as f64) as f32;
        let relative_y = y - floor_y as f64;
        let relative_z = (z - floor_z as f64) as f32;
        let sampled_y = match self.smear_y {
            None => relative_y as f32,
            Some(scale) => {
                let limit = if raw_y >= 0.0 && raw_y < relative_y { raw_y } else { relative_y };
                (relative_y - (limit / scale + 1.0E-7f32 as f64).floor() * scale) as f32
            }
        };
        self.sample_and_lerp(floor_x, floor_y, floor_z, relative_x, sampled_y, relative_z, relative_y as f32)
    }

    fn sample_and_lerp(&self, x: i32, y: i32, z: i32, rx: f32, ry: f32, rz: f32, original_ry: f32) -> f32 {
        let x0 = self.permute(x);
        let x1 = self.permute(x + 1);
        let xy00 = self.permute(x0 + y);
        let xy01 = self.permute(x0 + y + 1);
        let xy10 = self.permute(x1 + y);
        let xy11 = self.permute(x1 + y + 1);
        let d000 = grad_dot(self.permute(xy00 + z), rx, ry, rz);
        let d100 = grad_dot(self.permute(xy10 + z), rx - 1.0, ry, rz);
        let d010 = grad_dot(self.permute(xy01 + z), rx, ry - 1.0, rz);
        let d110 = grad_dot(self.permute(xy11 + z), rx - 1.0, ry - 1.0, rz);
        let d001 = grad_dot(self.permute(xy00 + z + 1), rx, ry, rz - 1.0);
        let d101 = grad_dot(self.permute(xy10 + z + 1), rx - 1.0, ry, rz - 1.0);
        let d011 = grad_dot(self.permute(xy01 + z + 1), rx, ry - 1.0, rz - 1.0);
        let d111 = grad_dot(self.permute(xy11 + z + 1), rx - 1.0, ry - 1.0, rz - 1.0);
        lerp3(smoothstep(rx), smoothstep(original_ry), smoothstep(rz), d000, d100, d010, d110, d001, d101, d011, d111)
    }
}
