pub struct BitArray<const N: usize> {
    bits: u8,
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
        Self {
            bits: 1,
            blocks: vec![0u32; N.div_ceil(32)].into_boxed_slice(),
        }
    }
    
    pub fn get_blocks(&self) -> &[u32] {
        &self.blocks
    }
    
    pub fn get_bits(&self) -> u8 {
        self.bits
    }

    fn per_word(&self) -> usize {
        32 / self.bits as usize
    }

    fn max_value(&self) -> u32 {
        (1 << self.bits) - 1
    }

    pub fn set(&mut self, index: usize, value: u16) {
        if value > self.max_value() as u16 {
            let bits = Self::bits_for(value);

            self.resize(bits);
        }

        let word = index / self.per_word();
        let offset = (index % self.per_word()) * self.bits as usize;

        let v = value as u32;
        let mask = self.max_value() << offset;

        self.blocks[word] = (self.blocks[word] & !mask) | ((v << offset) & mask);
    }

    pub fn get(&self, index: usize) -> u32 {
        let word = index / self.per_word();
        let offset = (index % self.per_word()) * self.bits as usize;
        (self.blocks[word] >> offset) & self.max_value()
    }

    fn bits_for(value: u16) -> u8 {
        Self::VALID_BITS.iter().copied().find(|&n| value < (1 << n)).unwrap_or(16)
    }

    fn resize(&mut self, bits: u8) {
        let old_bits = self.bits;
        let old_data = std::mem::take(&mut self.blocks);

        self.bits = bits;

        let per_word = self.per_word();
        let words = N.div_ceil(per_word);

        self.blocks = vec![0u32; words].into_boxed_slice();

        let old_per_word = 32 / old_bits as usize;
        let old_mask = (1 << old_bits) - 1 ;

        for i in 0..N {
            let old_word = i / old_per_word;
            let old_offset = (i % old_per_word) * old_bits as usize;
            let value = (old_data[old_word] >> old_offset) & old_mask;

            self.set(i, value as u16);
        }
    }
}