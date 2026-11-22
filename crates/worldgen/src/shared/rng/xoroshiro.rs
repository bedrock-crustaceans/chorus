use super::random::{BitSource, DOUBLE_UNIT, FLOAT_UNIT, RandomSource, block_seed, wide_block_seed};
use md5::{Digest, Md5};

const GOLDEN_RATIO_64: i64 = -7046029254386353131;
const SILVER_RATIO_64: i64 = 7640891576956012809;

pub fn mix_stafford13(mut z: i64) -> i64 {
    z = (z ^ ((z as u64) >> 30) as i64).wrapping_mul(-4658895280553007687);
    z = (z ^ ((z as u64) >> 27) as i64).wrapping_mul(-7723592293110705685);
    z ^ ((z as u64) >> 31) as i64
}

pub fn upgrade_seed_to_128bit(seed: i64) -> (i64, i64) {
    let lo = seed ^ SILVER_RATIO_64;
    let hi = lo.wrapping_add(GOLDEN_RATIO_64);
    (mix_stafford13(lo), mix_stafford13(hi))
}

pub fn seed_hash_of(input: &str) -> (i64, i64) {
    let hash = Md5::digest(input.as_bytes());
    let lo = i64::from_be_bytes(hash[0..8].try_into().unwrap());
    let hi = i64::from_be_bytes(hash[8..16].try_into().unwrap());
    (lo, hi)
}

#[derive(Clone)]
pub struct Xoroshiro {
    lo: i64,
    hi: i64,
}

impl Xoroshiro {
    pub fn new(seed: i64) -> Self {
        let (lo, hi) = upgrade_seed_to_128bit(seed);
        Self::from_parts(lo, hi)
    }

    pub fn from_parts(lo: i64, hi: i64) -> Self {
        if (lo | hi) == 0 {
            Self {
                lo: GOLDEN_RATIO_64,
                hi: SILVER_RATIO_64,
            }
        } else {
            Self { lo, hi }
        }
    }

    fn next_raw_bits(&mut self, bits: u32) -> i64 {
        ((self.next_long() as u64) >> (64 - bits)) as i64
    }

    pub fn fork_positional(&mut self) -> PositionalFactory {
        PositionalFactory {
            lo: self.next_long(),
            hi: self.next_long(),
        }
    }
}

impl RandomSource for Xoroshiro {
    fn next_long(&mut self) -> i64 {
        let s0 = self.lo;
        let mut s1 = self.hi;
        let result = s0.wrapping_add(s1).rotate_left(17).wrapping_add(s0);
        s1 ^= s0;
        self.lo = s0.rotate_left(49) ^ s1 ^ (s1 << 21);
        self.hi = s1.rotate_left(28);
        result
    }

    fn next_int(&mut self) -> i32 {
        self.next_long() as i32
    }

    fn next_int_bounded(&mut self, bound: i32) -> i32 {
        let bound64 = bound as u64;
        let mut bits = self.next_int() as u32 as u64;
        let mut multiplied = bits * bound64;
        let mut fractional = multiplied & 0xFFFF_FFFF;
        if fractional < bound64 {
            let threshold = ((bound as u32).wrapping_neg() % bound as u32) as u64;
            while fractional < threshold {
                bits = self.next_int() as u32 as u64;
                multiplied = bits * bound64;
                fractional = multiplied & 0xFFFF_FFFF;
            }
        }
        (multiplied >> 32) as i32
    }

    fn next_float(&mut self) -> f32 {
        self.next_raw_bits(24) as f32 * FLOAT_UNIT
    }

    fn next_double(&mut self) -> f64 {
        self.next_raw_bits(53) as f64 * DOUBLE_UNIT
    }
}

#[derive(Clone, Copy)]
pub struct PositionalFactory {
    lo: i64,
    hi: i64,
}

impl PositionalFactory {
    pub fn at(&self, x: i32, y: i32, z: i32) -> Xoroshiro {
        Xoroshiro::from_parts(block_seed(x, y, z) ^ self.lo, self.hi)
    }

    pub fn at_wide(&self, x: i32, y: i32, z: i32) -> Xoroshiro {
        Xoroshiro::from_parts(wide_block_seed(x, y, z) ^ self.lo, self.hi)
    }

    pub fn hash_of(&self, name: &str) -> Xoroshiro {
        let (lo, hi) = seed_hash_of(name);
        Xoroshiro::from_parts(lo ^ self.lo, hi ^ self.hi)
    }
}

impl BitSource for Xoroshiro {
    fn next_bits(&mut self, bits: u32) -> i32 {
        ((self.next_long() as u64) >> (64 - bits)) as i32
    }
}
