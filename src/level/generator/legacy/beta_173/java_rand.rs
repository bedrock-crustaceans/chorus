use std::num::Wrapping;

#[derive(Clone, Debug)]
pub struct JavaRand {
    seed: Wrapping<i64>,
}

const MULTIPLIER: Wrapping<i64> = Wrapping(0x5DEECE66D);
const ADDEND: Wrapping<i64> = Wrapping(0xB);
const MASK: Wrapping<i64> = Wrapping((1i64 << 48) - 1);

impl JavaRand {
    pub fn new(seed: i64) -> Self {
        let s = (Wrapping(seed) ^ MULTIPLIER) & MASK;
        JavaRand { seed: s }
    }

    pub fn next(&mut self, bits: u32) -> i64 {
        assert!(bits <= 48, "cannot return more than 48 bits");
        
        self.seed = (self.seed * MULTIPLIER + ADDEND) & MASK;
        self.seed.0 >> (48 - bits)
    }
    
    pub fn next_i32(&mut self) -> i32 {
        self.next(32) as i32
    }

    pub fn next_i32_bounded(&mut self, bound: i32) -> i32 {
        assert!(bound > 0, "bound must be positive");

        if (bound & -bound) == bound {
            return ((bound as i64 * self.next(31)) >> 31) as i32;
        }

        loop {
            let bits = self.next(31) as i32;
            let val = bits % bound;
            if bits - val + (bound - 1) >= 0 {
                return val;
            }
        }
    }

    pub fn next_i64(&mut self) -> i64 {
        let hi = self.next(32);
        let lo = self.next(32);
        (hi << 32) | lo
    }
    
    pub fn next_bool(&mut self) -> bool {
        self.next(1) != 0
    }

    pub fn next_float(&mut self) -> f32 {
        self.next(24) as f32 / ((1i32 << 24) as f32)
    }

    pub fn next_double(&mut self) -> f64 {
        let hi = self.next(26);
        let lo = self.next(27);
        (((hi << 27) | lo) as f64) / ((1i64 << 53) as f64)
    }
    
    pub fn get_seed(&self) -> i64 {
        self.seed.0    
    }
    
    pub fn set_seed(&mut self, seed: i64) {
        *self = JavaRand::new(seed);
    }
}