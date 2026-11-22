use super::mersenne::MersenneTwister;

const CHUNK_X_MULTIPLIER: i32 = 341872712;
const CHUNK_Z_MULTIPLIER: i32 = 132899541;
const REGION_X_MULTIPLIER: i32 = -1724254968;
const REGION_Z_MULTIPLIER: i32 = -245998635;
const MATERIAL_ADJUSTMENT_BURN: usize = 20202;

pub fn level_seed32(seed: i64) -> i32 {
    seed as i32
}

pub fn chunk_seed(chunk_x: i32, chunk_z: i32) -> u32 {
    chunk_x.wrapping_mul(CHUNK_X_MULTIPLIER).wrapping_add(chunk_z.wrapping_mul(CHUNK_Z_MULTIPLIER)) as u32
}

pub fn chunk_random(chunk_x: i32, chunk_z: i32) -> MersenneTwister {
    MersenneTwister::new(chunk_seed(chunk_x, chunk_z))
}

pub fn region_seed(level_seed: u32, region_x: i32, region_z: i32, salt: i32) -> u32 {
    region_z
        .wrapping_mul(REGION_Z_MULTIPLIER)
        .wrapping_add(region_x.wrapping_mul(REGION_X_MULTIPLIER))
        .wrapping_add(level_seed as i32)
        .wrapping_add(salt) as u32
}

pub fn material_adjustment_random(level_seed: i64) -> MersenneTwister {
    let mut random = MersenneTwister::new(level_seed32(level_seed) as u32);
    random.skip(MATERIAL_ADJUSTMENT_BURN);
    random
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chunk_and_region_seeds_wrap_in_32_bits() {
        assert_eq!(chunk_seed(1, 1), 341872712 + 132899541);
        assert_eq!(chunk_seed(-1, 0), 341872712u32.wrapping_neg());
        assert_eq!(region_seed(3, 0, 0, 2), 5);
        assert_eq!(region_seed(0, 1, 1, 0), (-245998635i32).wrapping_add(-1724254968) as u32);
        assert_eq!(level_seed32(-1), -1);
    }
}
