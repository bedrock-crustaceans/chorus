use crate::level::generator::legacy::beta_173::java_rand::JavaRand;
use crate::level::generator::legacy::beta_173::noise::perlin::PerlinNoise;

#[derive(Clone, Debug)]
pub struct OctaveNoise {
    octaves: Vec<PerlinNoise>,
    octaves_count: i32,
}

impl OctaveNoise {
    pub fn new(rng: &mut JavaRand, octaves_count: i32) -> Self {
        let mut octaves = Vec::with_capacity(octaves_count as usize);
        for _ in 0..octaves_count {
            octaves.push(PerlinNoise::new(rng));
        }
        OctaveNoise { octaves, octaves_count }
    }

    pub fn noise_3d(&self, x: f64, y: f64, z: f64, scale_x: f64, scale_y: f64, scale_z: f64) -> f64 {
        let mut result = 0.0;
        let mut frequency = 1.0;
        let mut amplitude = 1.0;

        for octave in &self.octaves {
            let nx = x * frequency * scale_x;
            let ny = y * frequency * scale_y;
            let nz = z * frequency * scale_z;

            result += octave.noise_3d(nx, ny, nz) * amplitude;

            frequency *= 2.0;
            amplitude /= 2.0;
        }

        result
    }
}
