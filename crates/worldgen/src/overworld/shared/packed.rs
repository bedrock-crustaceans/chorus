#[derive(Clone)]
pub struct Packed {
    bits: u32,
    words: Box<[u32]>,
}

impl Packed {
    pub fn bits_for(largest: usize) -> u32 {
        usize::BITS - largest.leading_zeros()
    }

    pub fn new(largest: usize, values: &[u16]) -> Self {
        let bits = Self::bits_for(largest);
        if bits == 0 {
            return Self { bits, words: Box::default() };
        }
        let per_word = (32 / bits) as usize;
        let mut words = vec![0u32; values.len().div_ceil(per_word)];
        for (index, &value) in values.iter().enumerate() {
            words[index / per_word] |= (value as u32) << ((index % per_word) as u32 * bits);
        }
        Self { bits, words: words.into_boxed_slice() }
    }

    pub fn get(&self, index: usize) -> u16 {
        if self.bits == 0 {
            return 0;
        }
        let per_word = (32 / self.bits) as usize;
        ((self.words[index / per_word] >> ((index % per_word) as u32 * self.bits)) & ((1 << self.bits) - 1)) as u16
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn values_come_back_at_every_width() {
        for largest in [0usize, 1, 2, 3, 5, 8, 31, 97, 255, 1000] {
            let values: Vec<u16> = (0..137).map(|i| (i * 7 % (largest + 1)) as u16).collect();
            let packed = Packed::new(largest, &values);
            for (index, &value) in values.iter().enumerate() {
                assert_eq!(packed.get(index), value, "largest {largest} index {index}");
            }
        }
    }

    #[test]
    fn a_single_value_takes_no_words() {
        assert_eq!(Packed::new(0, &[0; 50]).words.len(), 0);
    }
}
