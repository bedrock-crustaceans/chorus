const STATE_SIZE: usize = 624;
const SHIFT_SIZE: usize = 397;
const INIT_MULTIPLIER: u32 = 1812433253;
const MATRIX_A: u32 = 0x9908B0DF;
const UPPER_MASK: u32 = 0x8000_0000;
const LOWER_MASK: u32 = 0x7FFF_FFFF;
const TEMPER_B: u32 = 0x9D2C_5680;
const TEMPER_C: u32 = 0xEFC6_0000;
const UNSEEDED_SEED: u32 = 5489;
const UNIT_32: f64 = 2.3283064365386963e-10;

#[derive(Clone)]
pub struct MersenneTwister {
    seed: u32,
    state: [u32; STATE_SIZE],
    index: usize,
    spare_gaussian: Option<f32>,
}

impl Default for MersenneTwister {
    fn default() -> Self {
        Self::new(UNSEEDED_SEED)
    }
}

impl MersenneTwister {
    pub fn new(seed: u32) -> Self {
        let mut state = [0u32; STATE_SIZE];
        state[0] = seed;
        for i in 1..STATE_SIZE {
            let previous = state[i - 1];
            state[i] = INIT_MULTIPLIER.wrapping_mul(previous ^ (previous >> 30)).wrapping_add(i as u32);
        }
        Self {
            seed,
            state,
            index: STATE_SIZE,
            spare_gaussian: None,
        }
    }

    pub fn seed(&self) -> u32 {
        self.seed
    }

    pub fn leading_words<const COUNT: usize>(seed: u32) -> [u32; COUNT] {
        let mut state = [0u32; STATE_SIZE];
        state[0] = seed;
        for i in 1..SHIFT_SIZE + COUNT {
            let previous = state[i - 1];
            state[i] = INIT_MULTIPLIER.wrapping_mul(previous ^ (previous >> 30)).wrapping_add(i as u32);
        }
        std::array::from_fn(|k| {
            let y = (state[k] & UPPER_MASK) | (state[k + 1] & LOWER_MASK);
            let mag = if y & 1 == 1 { MATRIX_A } else { 0 };
            Self::temper(state[k + SHIFT_SIZE] ^ (y >> 1) ^ mag)
        })
    }

    fn temper(mut y: u32) -> u32 {
        y ^= y >> 11;
        y ^= (y << 7) & TEMPER_B;
        y ^= (y << 15) & TEMPER_C;
        y ^ (y >> 18)
    }

    pub fn next_gaussian(&mut self) -> f32 {
        if let Some(spare) = self.spare_gaussian.take() {
            return spare;
        }
        loop {
            let first = self.next_float() * 2.0 - 1.0;
            let second = self.next_float() * 2.0 - 1.0;
            let square = second * second + first * first;
            if square >= 1.0 || square == 0.0 {
                continue;
            }
            let multiplier = (square.ln() * -2.0 / square).sqrt();
            self.spare_gaussian = Some(second * multiplier);
            return first * multiplier;
        }
    }

    fn twist(&mut self) {
        for k in 0..STATE_SIZE {
            let y = (self.state[k] & UPPER_MASK) | (self.state[(k + 1) % STATE_SIZE] & LOWER_MASK);
            let mag = if y & 1 == 1 { MATRIX_A } else { 0 };
            self.state[k] = self.state[(k + SHIFT_SIZE) % STATE_SIZE] ^ (y >> 1) ^ mag;
        }
        self.index = 0;
    }

    pub fn next_u32(&mut self) -> u32 {
        if self.index >= STATE_SIZE {
            self.twist();
        }
        let y = self.state[self.index];
        self.index += 1;
        Self::temper(y)
    }

    pub fn skip(&mut self, count: usize) {
        for _ in 0..count {
            self.next_u32();
        }
    }

    pub fn next_int(&mut self) -> i32 {
        (self.next_u32() >> 1) as i32
    }

    pub fn next_int_bounded(&mut self, bound: i32) -> i32 {
        if bound == 0 { 0 } else { (self.next_u32() % bound as u32) as i32 }
    }

    pub fn next_boolean(&mut self) -> bool {
        (self.next_u32() >> 27) & 1 != 0
    }

    pub fn next_float(&mut self) -> f32 {
        Self::float_from_word(self.next_u32())
    }

    pub fn float_from_word(word: u32) -> f32 {
        (word as f64 * UNIT_32) as f32
    }

    pub fn next_double(&mut self) -> f64 {
        self.next_u32() as f64 * UNIT_32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    impl MersenneTwister {
        fn next_long(&mut self) -> i64 {
            let high = (self.next_u32() as i64) << 32;
            high.wrapping_add(self.next_u32() as i32 as i64)
        }
    }

    #[test]
    fn matches_reference_outputs() {
        let mut default = MersenneTwister::default();
        assert_eq!([default.next_u32(), default.next_u32(), default.next_u32()], [3499211612, 581869302, 3890346734]);
        let mut default = MersenneTwister::default();
        default.skip(9999);
        assert_eq!(default.next_u32(), 4123659995);
        assert_eq!(MersenneTwister::new(1).next_u32(), 1791095845);
    }

    #[test]
    fn leading_words_equal_the_first_outputs_of_the_full_generator() {
        for seed in (0..4000u32).map(|value| value.wrapping_mul(2_654_435_761)) {
            let mut full = MersenneTwister::new(seed);
            let expected = [full.next_u32(), full.next_u32(), full.next_u32(), full.next_u32()];
            assert_eq!(MersenneTwister::leading_words::<4>(seed), expected, "{seed}");
        }
    }

    #[test]
    fn derived_draws_follow_the_oracle() {
        let mut random = MersenneTwister::new(1);
        let mut words = MersenneTwister::new(1);
        assert_eq!(random.next_int(), (words.next_u32() >> 1) as i32);
        assert_eq!(random.next_int_bounded(10), (words.next_u32() % 10) as i32);
        assert_eq!(random.next_boolean(), (words.next_u32() >> 27) & 1 != 0);
        let high = (words.next_u32() as i64) << 32;
        assert_eq!(random.next_long(), high.wrapping_add(words.next_u32() as i32 as i64));
        assert_eq!(random.next_int_bounded(0), 0);
        assert_eq!(random.next_float(), (words.next_u32() as f64 * UNIT_32) as f32);
        assert_eq!(random.next_double(), words.next_u32() as f64 * UNIT_32);
    }
}
