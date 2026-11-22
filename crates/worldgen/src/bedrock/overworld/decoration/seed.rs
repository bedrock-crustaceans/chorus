use crate::shared::rng::mersenne::MersenneTwister;

const HASH_COMBINE_CONSTANT: u32 = 0x9E3779B9;
const HASH_COMBINE_LEFT: u32 = 6;
const HASH_COMBINE_RIGHT: u32 = 2;
const NEXT_INT_SHIFT: u32 = 1;

pub fn large_feature_multipliers(level_seed: u32) -> (u32, u32) {
    let mut random = MersenneTwister::new(level_seed);
    let mut odd_next_int = || random.next_u32() >> NEXT_INT_SHIFT | 1;
    (odd_next_int(), odd_next_int())
}

pub fn decoration_seed(level_seed: u32, chunk_x: i32, chunk_z: i32) -> u32 {
    let (x_multiplier, z_multiplier) = large_feature_multipliers(level_seed);
    level_seed ^ (chunk_z as u32).wrapping_mul(z_multiplier).wrapping_add((chunk_x as u32).wrapping_mul(x_multiplier))
}

pub fn rule_seed(identifier_hash: u64, decoration_seed: u32) -> u32 {
    let combined = (identifier_hash as u32)
        .wrapping_add(HASH_COMBINE_CONSTANT)
        .wrapping_add(decoration_seed << HASH_COMBINE_LEFT)
        .wrapping_add(decoration_seed >> HASH_COMBINE_RIGHT);
    combined ^ decoration_seed
}
