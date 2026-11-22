use crate::shared::rng::mersenne::MersenneTwister;

#[derive(Clone, Copy)]
pub struct LargeFeatureSeed {
    level_seed: u32,
    x_factor: i32,
    z_factor: i32,
}

fn force_odd(value: i32) -> i32 {
    value / 2 * 2 + 1
}

impl LargeFeatureSeed {
    pub fn new(level_seed: u32) -> Self {
        let mut random = MersenneTwister::new(level_seed);
        let x_factor = force_odd((random.next_u32() >> 1) as i32);
        let z_factor = force_odd((random.next_u32() >> 1) as i32);
        Self { level_seed, x_factor, z_factor }
    }

    pub fn chunk(&self, chunk_x: i32, chunk_z: i32) -> u32 {
        chunk_z.wrapping_mul(self.z_factor).wrapping_add(chunk_x.wrapping_mul(self.x_factor)) as u32 ^ self.level_seed
    }

    pub fn random(&self, chunk_x: i32, chunk_z: i32) -> MersenneTwister {
        MersenneTwister::new(self.chunk(chunk_x, chunk_z))
    }

    pub fn blueprint(&self, (chunk_x, chunk_z): (i32, i32)) -> MersenneTwister {
        let mut random = self.random(chunk_x, chunk_z);
        random.next_u32();
        random
    }
}
