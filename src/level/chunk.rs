use crate::level::sub_chunk::SubChunk;
use varint_rs::VarintWriter;

pub struct Chunk {
    pub x: i32,
    pub z: i32,
    sub_chunks: Vec<SubChunk>,
    min_sub_chunk_y: i8,
}

impl Chunk {
    pub fn new(x: i32, z: i32, min_sub_chunk_y: i8, count: usize, air_id: u32, biome: u32) -> Self {
        Self {
            x,
            z,
            sub_chunks: (0..count).map(|_| SubChunk::new(air_id, biome)).collect(),
            min_sub_chunk_y,
        }
    }

    fn sub_chunk_offset(&self, sub_y: i8) -> usize {
        sub_y.wrapping_sub(self.min_sub_chunk_y) as usize
    }

    pub fn get_sub_chunk(&self, sub_y: i8) -> Option<&SubChunk> {
        self.sub_chunks.get(self.sub_chunk_offset(sub_y))
    }

    pub fn get_sub_chunk_mut(&mut self, sub_y: i8) -> Option<&mut SubChunk> {
        let offset = self.sub_chunk_offset(sub_y);
        self.sub_chunks.get_mut(offset)
    }

    pub fn get_block(&self, x: u8, y: i32, z: u8, layer: usize) -> Option<u32> {
        let sub_y = (y >> 4) as i8;
        let local_y = (y & 0xF) as u8;
        Some(self.get_sub_chunk(sub_y)?.get(x, local_y, z, layer))
    }

    pub fn set_block(&mut self, x: u8, y: i32, z: u8, layer: usize, block_id: u32) -> bool {
        let sub_y = (y >> 4) as i8;
        let local_y = (y & 0xF) as u8;
        match self.get_sub_chunk_mut(sub_y) {
            Some(sc) => { sc.set(x, local_y, z, layer, block_id); true }
            None => false,
        }
    }

    pub fn highest_non_air_sub_chunk_y(&self) -> i8 {
        for i in (0..self.sub_chunks.len()).rev() {
            if !self.sub_chunks[i].is_all_air() {
                return self.min_sub_chunk_y.wrapping_add(i as i8);
            }
        }
        self.min_sub_chunk_y
    }

    /// Serializes biome data for LevelChunkPacket.serialized_chunk_data.
    pub fn serialize_biomes(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        for sc in &self.sub_chunks {
            buf.push(0x01u8);  // V0 single-entry network header
            buf.write_u32_varint(sc.biome).unwrap();
        }
        buf.push(0x00);  // border block count
        buf
    }
}
