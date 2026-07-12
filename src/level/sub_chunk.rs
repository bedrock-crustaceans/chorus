use bedrock::protocol::ProtoCodec;
use crate::level::palette::Palette;

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
