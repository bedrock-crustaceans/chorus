use md5::{Digest, Md5};

const GOLDEN_RATIO_64: i64 = -7046029254386353131;
const SILVER_RATIO_64: i64 = 7640891576956012809;
const FLOAT_UNIT: f32 = 5.9604645E-8;
const DOUBLE_UNIT: f64 = 1.110223E-16f32 as f64;

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

pub fn block_seed(x: i32, y: i32, z: i32) -> i64 {
    let mut seed = (x.wrapping_mul(3129871) as i64) ^ (z as i64).wrapping_mul(116129781) ^ y as i64;
    seed = seed.wrapping_mul(seed).wrapping_mul(42317861).wrapping_add(seed.wrapping_mul(11));
    seed >> 16
}

pub trait RandomSource {
    fn next_long(&mut self) -> i64;
    fn next_int(&mut self) -> i32;
    fn next_int_bounded(&mut self, bound: i32) -> i32;
    fn next_float(&mut self) -> f32;
    fn next_double(&mut self) -> f64;
    fn next_boolean(&mut self) -> bool;
    fn set_seed(&mut self, seed: i64);
    fn gaussian_slot(&mut self) -> &mut Option<f64>;

    fn next_gaussian(&mut self) -> f64 {
        if let Some(value) = self.gaussian_slot().take() {
            return value;
        }
        loop {
            let x = 2.0 * self.next_double() - 1.0;
            let y = 2.0 * self.next_double() - 1.0;
            let radius_squared = x * x + y * y;
            if radius_squared < 1.0 && radius_squared != 0.0 {
                let multiplier = (-2.0 * radius_squared.ln() / radius_squared).sqrt();
                *self.gaussian_slot() = Some(y * multiplier);
                return x * multiplier;
            }
        }
    }

    fn consume_count(&mut self, count: usize) {
        for _ in 0..count {
            self.next_int();
        }
    }

    fn next_int_between_inclusive(&mut self, min: i32, max: i32) -> i32 {
        self.next_int_bounded(max - min + 1) + min
    }

    fn triangle(&mut self, mean: f64, spread: f64) -> f64 {
        mean + spread * (self.next_double() - self.next_double())
    }
}

#[derive(Clone)]
pub struct Xoroshiro {
    lo: i64,
    hi: i64,
    gaussian: Option<f64>,
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
                gaussian: None,
            }
        } else {
            Self { lo, hi, gaussian: None }
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

    fn next_boolean(&mut self) -> bool {
        (self.next_long() & 1) != 0
    }

    fn set_seed(&mut self, seed: i64) {
        *self = Self::new(seed);
    }

    fn gaussian_slot(&mut self) -> &mut Option<f64> {
        &mut self.gaussian
    }

    fn consume_count(&mut self, count: usize) {
        for _ in 0..count {
            self.next_long();
        }
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

    pub fn hash_of(&self, name: &str) -> Xoroshiro {
        let (lo, hi) = seed_hash_of(name);
        Xoroshiro::from_parts(lo ^ self.lo, hi ^ self.hi)
    }
}

pub trait BitSource {
    fn next_bits(&mut self, bits: u32) -> i32;
}

impl BitSource for Xoroshiro {
    fn next_bits(&mut self, bits: u32) -> i32 {
        ((self.next_long() as u64) >> (64 - bits)) as i32
    }
}

pub trait LegacyBits: BitSource {
    fn reseed(&mut self, seed: i64);
    fn gaussian(&mut self) -> &mut Option<f64>;
}

impl<T: LegacyBits> RandomSource for T {
    fn next_long(&mut self) -> i64 {
        let upper = self.next_bits(32) as i64;
        let lower = self.next_bits(32) as i64;
        (upper << 32).wrapping_add(lower)
    }

    fn next_int(&mut self) -> i32 {
        self.next_bits(32)
    }

    fn next_int_bounded(&mut self, bound: i32) -> i32 {
        if (bound & (bound - 1)) == 0 {
            return ((bound as i64 * self.next_bits(31) as i64) >> 31) as i32;
        }
        loop {
            let sample = self.next_bits(31);
            let modulo = sample % bound;
            if sample.wrapping_sub(modulo).wrapping_add(bound - 1) >= 0 {
                return modulo;
            }
        }
    }

    fn next_float(&mut self) -> f32 {
        self.next_bits(24) as f32 * FLOAT_UNIT
    }

    fn next_double(&mut self) -> f64 {
        let upper = self.next_bits(26) as i64;
        let lower = self.next_bits(27) as i64;
        ((upper << 27) + lower) as f64 * DOUBLE_UNIT
    }

    fn next_boolean(&mut self) -> bool {
        self.next_bits(1) != 0
    }

    fn set_seed(&mut self, seed: i64) {
        self.reseed(seed);
    }

    fn gaussian_slot(&mut self) -> &mut Option<f64> {
        self.gaussian()
    }
}

#[derive(Clone)]
pub struct Legacy {
    seed: i64,
    gaussian: Option<f64>,
}

impl Legacy {
    const MULTIPLIER: i64 = 25214903917;
    const MASK: i64 = (1 << 48) - 1;

    pub fn new(seed: i64) -> Self {
        Self {
            seed: (seed ^ Self::MULTIPLIER) & Self::MASK,
            gaussian: None,
        }
    }
}

impl BitSource for Legacy {
    fn next_bits(&mut self, bits: u32) -> i32 {
        self.seed = self.seed.wrapping_mul(Self::MULTIPLIER).wrapping_add(11) & Self::MASK;
        (self.seed >> (48 - bits)) as i32
    }
}

impl LegacyBits for Legacy {
    fn reseed(&mut self, seed: i64) {
        *self = Self::new(seed);
    }

    fn gaussian(&mut self) -> &mut Option<f64> {
        &mut self.gaussian
    }
}

pub struct WorldgenRandom<R: RandomSource + BitSource> {
    pub source: R,
    gaussian: Option<f64>,
}

impl<R: RandomSource + BitSource> WorldgenRandom<R> {
    pub fn new(source: R) -> Self {
        Self { source, gaussian: None }
    }

    pub fn set_decoration_seed(&mut self, seed: i64, x: i32, z: i32) -> i64 {
        self.set_seed(seed);
        let x_scale = self.next_long() | 1;
        let z_scale = self.next_long() | 1;
        let result = (x as i64).wrapping_mul(x_scale).wrapping_add((z as i64).wrapping_mul(z_scale)) ^ seed;
        self.set_seed(result);
        result
    }

    pub fn set_feature_seed(&mut self, seed: i64, index: i32, step: i32) {
        self.set_seed(seed.wrapping_add(index as i64).wrapping_add(10000 * step as i64));
    }

    pub fn set_large_feature_seed(&mut self, seed: i64, x: i32, z: i32) {
        self.set_seed(seed);
        let x_scale = self.next_long();
        let z_scale = self.next_long();
        let result = (x as i64).wrapping_mul(x_scale) ^ (z as i64).wrapping_mul(z_scale) ^ seed;
        self.set_seed(result);
    }
}

impl<R: RandomSource + BitSource> BitSource for WorldgenRandom<R> {
    fn next_bits(&mut self, bits: u32) -> i32 {
        self.source.next_bits(bits)
    }
}

impl<R: RandomSource + BitSource> LegacyBits for WorldgenRandom<R> {
    fn reseed(&mut self, seed: i64) {
        self.source.set_seed(seed);
    }

    fn gaussian(&mut self) -> &mut Option<f64> {
        &mut self.gaussian
    }
}
