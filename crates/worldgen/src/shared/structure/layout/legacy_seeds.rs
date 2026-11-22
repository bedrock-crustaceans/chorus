use crate::shared::rng::legacy::Legacy;
use crate::shared::rng::random::{RandomSource, block_seed};
use crate::shared::structure::Rotation;

pub const CANDIDATE_ROTATIONS: [Rotation; 4] = [Rotation::Clockwise90, Rotation::Clockwise180, Rotation::CounterClockwise90, Rotation::None];

const X_MULTIPLIER: i64 = 3_129_871;
const Z_MULTIPLIER: i64 = 116_129_781;
const MIX_MULTIPLIER: i64 = 42_317_861;
const MIX_ADDEND: i64 = 11;
const MIX_SHIFT: u32 = 16;

pub fn start_random(seed: i64, (chunk_x, chunk_z): (i32, i32)) -> Legacy {
    let mut base = Legacy::new(seed);
    let x_scale = base.next_long();
    let z_scale = base.next_long();
    Legacy::new((chunk_x as i64).wrapping_mul(x_scale) ^ (chunk_z as i64).wrapping_mul(z_scale) ^ seed)
}

pub fn forked_at(seed: i64, (x, y, z): (i32, i32, i32)) -> Legacy {
    let salt = Legacy::new(seed).next_long();
    Legacy::new(block_seed(x, y, z) ^ salt)
}

pub fn wide_position_seed((x, y, z): (i32, i32, i32)) -> i64 {
    let mixed = (x as i64).wrapping_mul(X_MULTIPLIER) ^ (z as i64).wrapping_mul(Z_MULTIPLIER) ^ y as i64;
    mixed.wrapping_mul(mixed).wrapping_mul(MIX_MULTIPLIER).wrapping_add(mixed.wrapping_mul(MIX_ADDEND)) >> MIX_SHIFT
}

pub fn shuffle<T>(random: &mut Legacy, items: &mut [T]) {
    for end in (2..=items.len()).rev() {
        let pick = random.next_int_bounded(end as i32) as usize;
        items.swap(end - 1, pick);
    }
}
