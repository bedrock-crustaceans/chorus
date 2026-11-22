pub(super) const FLOAT_UNIT: f32 = 5.9604645E-8;
pub(super) const DOUBLE_UNIT: f64 = 1.110223E-16f32 as f64;

pub fn block_seed(x: i32, y: i32, z: i32) -> i64 {
    let mut seed = (x.wrapping_mul(3129871) as i64) ^ (z as i64).wrapping_mul(116129781) ^ y as i64;
    seed = seed.wrapping_mul(seed).wrapping_mul(42317861).wrapping_add(seed.wrapping_mul(11));
    seed >> 16
}

pub fn wide_block_seed(x: i32, y: i32, z: i32) -> i64 {
    let mut seed = (x as i64).wrapping_mul(3129871) ^ (z as i64).wrapping_mul(116129781) ^ y as i64;
    seed = seed.wrapping_mul(seed).wrapping_mul(42317861).wrapping_add(seed.wrapping_mul(11));
    seed >> 16
}

pub trait RandomSource {
    fn next_long(&mut self) -> i64;
    fn next_int(&mut self) -> i32;
    fn next_int_bounded(&mut self, bound: i32) -> i32;
    fn next_float(&mut self) -> f32;
    fn next_double(&mut self) -> f64;
}

pub trait BitSource {
    fn next_bits(&mut self, bits: u32) -> i32;
}
