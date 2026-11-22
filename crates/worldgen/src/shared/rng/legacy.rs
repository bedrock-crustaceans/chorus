use super::random::{BitSource, DOUBLE_UNIT, FLOAT_UNIT, RandomSource};

pub trait LegacyBits: BitSource {}

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
}

#[derive(Clone)]
pub struct Legacy {
    seed: i64,
}

impl Legacy {
    const MULTIPLIER: i64 = 25214903917;
    const MASK: i64 = (1 << 48) - 1;

    pub fn new(seed: i64) -> Self {
        Self {
            seed: (seed ^ Self::MULTIPLIER) & Self::MASK,
        }
    }
}

impl BitSource for Legacy {
    fn next_bits(&mut self, bits: u32) -> i32 {
        self.seed = self.seed.wrapping_mul(Self::MULTIPLIER).wrapping_add(11) & Self::MASK;
        (self.seed >> (48 - bits)) as i32
    }
}

impl LegacyBits for Legacy {}
