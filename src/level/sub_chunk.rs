use varint_rs::VarintWriter;

const VALID_BITS: [u8; 8] = [1, 2, 3, 4, 5, 6, 8, 16];

fn sub_chunk_index(x: u8, y: u8, z: u8) -> usize {
    ((x as usize) << 8) | ((z as usize) << 4) | (y as usize)
}

pub struct PalettedLayer {
    palette: Vec<i32>,
    indices: Option<Box<[u16; 4096]>>,
}

impl PalettedLayer {
    pub fn new(value: i32) -> Self {
        Self {
            palette: vec![value],
            indices: None,
        }
    }

    pub fn get(&self, index: usize) -> i32 {
        match &self.indices {
            None => self.palette[0],
            Some(indices) => self.palette[indices[index] as usize],
        }
    }

    pub fn set(&mut self, index: usize, value: i32) {
        let palette_index = match self.palette.iter().position(|&v| v == value) {
            Some(i) => i,
            None => {
                self.palette.push(value);
                self.palette.len() - 1
            }
        };

        if palette_index == 0 && self.indices.is_none() {
            return;
        }

        let indices = self.indices.get_or_insert_with(|| Box::new([0u16; 4096]));
        indices[index] = palette_index as u16;
    }

    pub fn serialize(&self, buf: &mut Vec<u8>) {
        match &self.indices {
            None => {
                buf.push(0x01);
                buf.write_i32_varint(self.palette[0]).unwrap();
            }
            Some(indices) => {
                let bits = VALID_BITS.iter().copied().find(|&b| (1usize << b) >= self.palette.len()).unwrap_or(16);

                buf.push((bits << 1) | 1);

                let entries_per_word = (32 / bits) as usize;
                let word_count = (4096 + entries_per_word - 1) / entries_per_word;
                let mut words = vec![0u32; word_count];
                for (i, &idx) in indices.iter().enumerate() {
                    let word = i / entries_per_word;
                    let bit_offset = (i % entries_per_word) * bits as usize;
                    words[word] |= (idx as u32) << bit_offset;
                }
                for word in &words {
                    buf.extend_from_slice(&word.to_le_bytes());
                }

                buf.write_i32_varint(self.palette.len() as i32).unwrap();
                for &id in &self.palette {
                    buf.write_i32_varint(id).unwrap();
                }
            }
        }
    }
}

pub struct SubChunk {
    blocks: [PalettedLayer; 2],
    biomes: PalettedLayer,
    air_id: i32,
    non_air_count: u32,
}

impl SubChunk {
    pub fn new(air_id: i32, biome: i32) -> Self {
        Self {
            blocks: [PalettedLayer::new(air_id), PalettedLayer::new(air_id)],
            biomes: PalettedLayer::new(biome),
            air_id,
            non_air_count: 0,
        }
    }

    pub fn get(&self, x: u8, y: u8, z: u8, layer: usize) -> i32 {
        debug_assert!(x < 16 && y < 16 && z < 16 && layer < 2);
        self.blocks[layer].get(sub_chunk_index(x, y, z))
    }

    pub fn set(&mut self, x: u8, y: u8, z: u8, layer: usize, block_id: i32) {
        debug_assert!(x < 16 && y < 16 && z < 16 && layer < 2);
        let index = sub_chunk_index(x, y, z);

        let old = self.blocks[layer].get(index);
        if old == block_id {
            return;
        }

        if old == self.air_id {
            self.non_air_count += 1;
        } else if block_id == self.air_id {
            self.non_air_count -= 1;
        }

        self.blocks[layer].set(index, block_id);
    }

    pub fn is_all_air(&self) -> bool {
        self.non_air_count == 0
    }

    pub fn serialize_network(&self, sub_chunk_y: i8) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.push(9u8); // version = Limitless
        buf.push(2u8); // layer count
        buf.push(sub_chunk_y as u8);
        self.blocks[0].serialize(&mut buf);
        self.blocks[1].serialize(&mut buf);
        buf
    }

    pub fn serialize_biomes(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        self.biomes.serialize(&mut buf);
        buf
    }
}
