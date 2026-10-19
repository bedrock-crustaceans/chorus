#[derive(Clone)]
pub struct BitArray<const N: usize> {
    bits: u8,
    per_word: u32,
    mask: u32,
    blocks: Box<[u32]>,
}

impl<const N: usize> Default for BitArray<N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const N: usize> BitArray<N> {
    const VALID_BITS: [u8; 8] = [1, 2, 3, 4, 5, 6, 8, 16];

    pub fn new() -> Self {
        Self::with_bits(1)
    }

    pub fn with_bits(bits: u8) -> Self {
        Self {
            bits,
            per_word: 32 / bits as u32,
            mask: ((1u64 << bits) - 1) as u32,
            blocks: vec![0u32; N.div_ceil(32 / bits as usize)].into_boxed_slice(),
        }
    }

    pub fn from_indices(bits: u8, indices: &[u16; N]) -> Self {
        let mut array = Self::with_bits(bits);
        let per_word = array.per_word();
        for (word, group) in array.blocks.iter_mut().zip(indices.chunks(per_word)) {
            *word = group.iter().enumerate().fold(0u32, |packed, (slot, &index)| packed | (index as u32) << (slot * bits as usize));
        }
        array
    }

    pub fn indices(&self) -> impl Iterator<Item = u16> + '_ {
        let (bits, mask) = (self.bits as u32, self.mask);
        self.blocks
            .iter()
            .flat_map(move |&word| (0..self.per_word).map(move |slot| ((word >> (slot * bits)) & mask) as u16))
            .take(N)
    }

    pub fn get_blocks(&self) -> &[u32] {
        &self.blocks
    }

    pub fn get_bits(&self) -> u8 {
        self.bits
    }

    fn per_word(&self) -> usize {
        self.per_word as usize
    }

    fn max_value(&self) -> u32 {
        self.mask
    }

    pub fn set(&mut self, index: usize, value: u16) -> u16 {
        if value > self.max_value() as u16 {
            let bits = Self::bits_for(value);

            self.resize(bits);
        }

        let word = index / self.per_word();
        let offset = (index % self.per_word()) * self.bits as usize;

        let v = value as u32;
        let mask = self.max_value() << offset;

        let old = (self.blocks[word] & mask) >> offset;

        self.blocks[word] = (self.blocks[word] & !mask) | ((v << offset) & mask);

        old as u16
    }

    pub fn get(&self, index: usize) -> u16 {
        let word = index / self.per_word();
        let offset = (index % self.per_word()) * self.bits as usize;
        let val = (self.blocks[word] >> offset) & self.max_value();

        val as u16
    }

    pub fn bits_for(value: u16) -> u8 {
        Self::VALID_BITS.iter().copied().find(|&n| (value as u32) < (1u32 << n)).unwrap_or(16)
    }

    pub fn resize(&mut self, bits: u8) {
        let old_bits = self.bits;
        let old_data = std::mem::take(&mut self.blocks);

        self.bits = bits;
        self.per_word = 32 / bits as u32;
        self.mask = ((1u64 << bits) - 1) as u32;

        let per_word = self.per_word();
        let words = N.div_ceil(per_word);

        self.blocks = vec![0u32; words].into_boxed_slice();

        let old_per_word = 32 / old_bits as usize;
        let old_mask = (1 << old_bits) - 1;

        for i in 0..N {
            let old_word = i / old_per_word;
            let old_offset = (i % old_per_word) * old_bits as usize;
            let value = (old_data[old_word] >> old_offset) & old_mask;

            self.set(i, value as u16);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::Lcg;

    const WIDTHS: [u8; 8] = [1, 2, 3, 4, 5, 6, 8, 16];

    #[test]
    fn bits_for_picks_the_smallest_valid_width() {
        let cases = [
            (0, 1),
            (1, 1),
            (2, 2),
            (3, 2),
            (4, 3),
            (7, 3),
            (8, 4),
            (15, 4),
            (16, 5),
            (31, 5),
            (32, 6),
            (63, 6),
            (64, 8),
            (255, 8),
            (256, 16),
            (u16::MAX, 16),
        ];
        for (value, bits) in cases {
            assert_eq!(BitArray::<4096>::bits_for(value), bits, "value {value}");
        }
    }

    #[test]
    fn word_count_matches_the_packing_of_each_width() {
        for bits in WIDTHS {
            let array = BitArray::<4096>::with_bits(bits);
            assert_eq!(array.get_blocks().len(), 4096usize.div_ceil(32 / bits as usize), "bits {bits}");
        }
    }

    #[test]
    fn every_width_stores_and_returns_random_values() {
        for bits in WIDTHS {
            let mut rng = Lcg::new(bits as u64);
            let max = (1u32 << bits) - 1;
            let expected: Vec<u16> = (0..4096).map(|_| rng.below(max + 1) as u16).collect();

            let mut array = BitArray::<4096>::with_bits(bits);
            for (i, &value) in expected.iter().enumerate() {
                array.set(i, value);
            }

            assert_eq!(array.get_bits(), bits);
            for (i, &value) in expected.iter().enumerate() {
                assert_eq!(array.get(i), value, "bits {bits} index {i}");
            }
        }
    }

    #[test]
    fn every_width_holds_its_maximum_value_without_resizing() {
        for bits in WIDTHS {
            let mut array = BitArray::<4096>::with_bits(bits);
            let max = ((1u32 << bits) - 1) as u16;
            array.set(4095, max);
            array.set(0, max);
            assert_eq!(array.get_bits(), bits);
            assert_eq!((array.get(0), array.get(1), array.get(4095)), (max, 0, max));
        }
    }

    #[test]
    fn from_indices_and_indices_round_trip_at_every_width() {
        for bits in [1u8, 2, 3, 4, 5, 6, 8, 16] {
            let max = ((1u32 << bits) - 1) as u16;
            let source: [u16; 4096] = std::array::from_fn(|i| (i as u32 * 7 % (max as u32 + 1)) as u16);
            let array = BitArray::<4096>::from_indices(bits, &source);
            assert_eq!(array.indices().collect::<Vec<_>>(), source);
            for (i, &value) in source.iter().enumerate() {
                assert_eq!(array.get(i), value);
            }
        }
    }

    #[test]
    fn set_returns_the_previous_value() {
        let mut array = BitArray::<4096>::new();
        assert_eq!(array.set(10, 1), 0);
        assert_eq!(array.set(10, 0), 1);
    }

    #[test]
    fn set_neighbours_do_not_bleed_into_each_other() {
        for bits in WIDTHS {
            let mut array = BitArray::<4096>::with_bits(bits);
            let max = ((1u32 << bits) - 1) as u16;
            array.set(5, max);
            assert_eq!((array.get(4), array.get(5), array.get(6)), (0, max, 0), "bits {bits}");
            array.set(5, 0);
            assert_eq!(array.get(5), 0);
        }
    }

    #[test]
    fn a_value_that_does_not_fit_grows_the_array_and_keeps_old_entries() {
        let mut rng = Lcg::new(7);
        let expected: Vec<u16> = (0..4096).map(|_| rng.below(2) as u16).collect();
        let mut array = BitArray::<4096>::new();
        for (i, &value) in expected.iter().enumerate() {
            array.set(i, value);
        }

        for (step, value) in [(2u8, 3u16), (3, 6), (4, 12), (5, 20), (6, 40), (8, 200), (16, 1000)] {
            array.set(100, value);
            assert_eq!(array.get_bits(), step);
            assert_eq!(array.get(100), value);
            for (i, &old) in expected.iter().enumerate().filter(|(i, _)| *i != 100) {
                assert_eq!(array.get(i), old, "after growing to {step} bits, index {i}");
            }
        }
    }

    #[test]
    fn resizing_down_keeps_values_that_fit() {
        let mut array = BitArray::<4096>::with_bits(8);
        for i in 0..4096 {
            array.set(i, (i % 4) as u16);
        }
        array.resize(2);
        assert_eq!(array.get_bits(), 2);
        for i in 0..4096 {
            assert_eq!(array.get(i), (i % 4) as u16);
        }
    }
}
