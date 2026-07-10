use bedrock::protocol::error::ProtoCodecError;
use bedrock::protocol::{ProtoCodec, ProtoCodecLE, ProtoCodecVAR};
use indexmap::IndexSet;
use std::io::{Read, Write};
use vek::num_traits::Zero;

pub struct Palette {
    palette: IndexSet<i32>,
    indices: Option<Box<[u16; 4096]>>,
}

impl Palette {
    pub const VALID_BITS: [u8; 8] = [1, 2, 3, 4, 5, 6, 8, 16];

    pub fn new(value: i32) -> Self {
        Self {
            palette: {
                let mut set = IndexSet::new();
                set.insert(value);
                set
            },
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
        let (palette_index, _) = self.palette.insert_full(value);

        if palette_index.is_zero() && self.indices.is_none() {
            return;
        }

        let indices = self.indices.get_or_insert_with(|| Box::new([0u16; 4096]));
        indices[index] = palette_index as u16;
    }
}

impl ProtoCodec for Palette {
    fn serialize<W: Write>(&self, stream: &mut W) -> Result<(), ProtoCodecError> {
        match &self.indices {
            None => {
                u8::serialize(&0x01, stream)?;
                <i32 as ProtoCodecVAR>::serialize(&self.palette[0], stream)?;
            }
            Some(indices) => {
                let bits = *Self::VALID_BITS.iter().find(|&b| (1usize << b) >= self.palette.len()).unwrap_or(&16);

                u8::serialize(&((bits << 1) | 1), stream)?;

                let entries_per_word = (32 / bits) as usize;
                let word_count = (4096 + entries_per_word - 1) / entries_per_word;
                let mut words = vec![0u32; word_count];
                for (i, &idx) in indices.iter().enumerate() {
                    let word = i / entries_per_word;
                    let bit_offset = (i % entries_per_word) * bits as usize;
                    words[word] |= (idx as u32) << bit_offset;
                }
                for word in &words {
                    <u32 as ProtoCodecLE>::serialize(word, stream)?;
                }

                <i32 as ProtoCodecVAR>::serialize(&(self.palette.len() as i32), stream)?;
                for id in &self.palette {
                    <i32 as ProtoCodecVAR>::serialize(id, stream)?;
                }
            }
        }
        Ok(())
    }

    fn deserialize<R: Read>(_stream: &mut R) -> Result<Self, ProtoCodecError> {
        unimplemented!()
    }

    fn size_hint(&self) -> usize {
        unimplemented!()
    }
}

pub struct SubChunk {
    blocks: Vec<Palette>,
    biomes: Palette,
    air_id: i32,
    non_air_count: u32,
}

impl SubChunk {
    pub fn new(air_id: i32, biome: i32) -> Self {
        Self {
            blocks: vec![Palette::new(air_id), Palette::new(air_id)],
            biomes: Palette::new(biome),
            air_id,
            non_air_count: 0,
        }
    }

    pub fn get(&self, x: u8, y: u8, z: u8, layer: usize) -> i32 {
        debug_assert!(x < 16 && y < 16 && z < 16 && layer < 2);
        self.blocks[layer].get(Self::index(x, y, z))
    }

    pub fn set(&mut self, x: u8, y: u8, z: u8, layer: usize, block_id: i32) {
        debug_assert!(x < 16 && y < 16 && z < 16 && layer < 2);
        let index = Self::index(x, y, z);

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
        buf.push(self.blocks.len() as u8);
        buf.push(sub_chunk_y as u8);
        for palette in &self.blocks {
            palette.serialize(&mut buf).unwrap();
        }
        buf
    }

    pub fn serialize_biomes(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        self.biomes.serialize(&mut buf).unwrap();
        buf
    }

    fn index(x: u8, y: u8, z: u8) -> usize {
        ((x as usize) << 8) | ((z as usize) << 4) | (y as usize)
    }
}
