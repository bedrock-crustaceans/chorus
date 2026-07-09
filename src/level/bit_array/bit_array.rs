use super::bit_array_version::BitArrayVersion;

pub struct BitArray {
    pub version: BitArrayVersion,
    pub size: usize,
    pub chunks: Vec<u32>,
}

impl BitArray {
    pub fn new(version: BitArrayVersion, size: usize) -> Self {
        Self { version, size, chunks: vec![] }
    }

    fn chunk_count(&self) -> usize {
        (self.size as f32 / self.version.entries() as f32).ceil() as usize
    }

    fn mask(&self) -> u32 {
        self.version.max_value()
    }

    pub fn get(&self, index: usize) -> u32 {
        match self.version.is_padded() {
            true => {
                let word = index / self.version.entries();
                let offset = (index % self.version.entries()) * self.version.bits();
                (self.chunks[word] >> offset) & self.mask()
            }
            false => {
                let bit_index = index * self.version.bits();
                let word = bit_index >> 5;
                let offset = bit_index & 31;
                (self.chunks[word] >> offset) & self.mask()
            }
        }
    }

    pub fn set(&mut self, index: usize, value: u32) {
        match self.version.is_padded() {
            true => {
                let word = index / self.version.entries();
                let offset = (index % self.version.entries()) * self.version.bits();
                let mask = self.mask() << offset;
                self.chunks[word] = (self.chunks[word] & !mask) | ((value & self.mask()) << offset);
            }
            false => {
                let bit_index = index * self.version.bits();
                let word = bit_index >> 5;
                let offset = bit_index & 31;
                let mask = self.mask() << offset;
                self.chunks[word] = (self.chunks[word] & !mask) | ((value & self.mask()) << offset);
            }
        }
    }
}
