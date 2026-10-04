use crate::sub_chunk::SubChunk;

#[derive(Clone)]
pub struct Chunk {
    pub x: i32,
    pub z: i32,
    sub_chunks: Vec<SubChunk>,
    min_sub_chunk_y: i8,
    block_entities: Vec<BlockEntity>,
    dirty: bool,
}

#[derive(Clone)]
pub struct BlockEntity {
    pub y: i32,
    pub data: nbtx::Value,
}

impl Chunk {
    pub fn new(x: i32, z: i32, min_sub_chunk_y: i8, count: usize, air_id: i32, biome: i32) -> Self {
        Self {
            x,
            z,
            sub_chunks: (0..count).map(|_| SubChunk::new(air_id, biome)).collect(),
            min_sub_chunk_y,
            block_entities: Vec::new(),
            dirty: false,
        }
    }

    pub fn from_sub_chunks(x: i32, z: i32, min_sub_chunk_y: i8, sub_chunks: Vec<SubChunk>, block_entities: Vec<BlockEntity>) -> Self {
        Self {
            x,
            z,
            sub_chunks,
            min_sub_chunk_y,
            block_entities,
            dirty: false,
        }
    }

    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    pub fn mark_saved(&mut self) {
        self.dirty = false;
    }

    pub fn serialize_block_entities(&self, sub_y: Option<i8>) -> Vec<u8> {
        let mut buf = Vec::new();
        for entity in self.block_entities.iter().filter(|entity| sub_y.is_none_or(|sub_y| (entity.y >> 4) as i8 == sub_y)) {
            buf.extend(nbtx::to_varint_bytes(&entity.data).expect("block entity nbt serializes"));
        }
        buf
    }

    pub fn set_biome(&mut self, x: u8, y: i32, z: u8, biome: i32) -> bool {
        match self.get_sub_chunk_mut((y >> 4) as i8) {
            Some(sub_chunk) => {
                sub_chunk.set_biome(x, (y & 0xF) as u8, z, biome);
                true
            }
            None => false,
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
        let sub_chunk = self.sub_chunks.get_mut(offset);
        self.dirty |= sub_chunk.is_some();
        sub_chunk
    }

    pub fn get_block(&self, x: u8, y: i32, z: u8, layer: usize) -> Option<i32> {
        let sub_y = (y >> 4) as i8;
        let local_y = (y & 0xF) as u8;
        Some(self.get_sub_chunk(sub_y)?.get(x, local_y, z, layer))
    }

    pub fn get_biome(&self, x: u8, y: i32, z: u8) -> Option<i32> {
        Some(self.get_sub_chunk((y >> 4) as i8)?.get_biome(x, (y & 0xF) as u8, z))
    }

    pub fn set_block(&mut self, x: u8, y: i32, z: u8, layer: usize, block_id: i32) -> bool {
        let offset = self.sub_chunk_offset((y >> 4) as i8);
        let changed = self.sub_chunks.get_mut(offset).is_some_and(|sub_chunk| sub_chunk.set(x, (y & 0xF) as u8, z, layer, block_id));
        self.dirty |= changed;
        changed
    }

    pub fn highest_non_air_sub_chunk_y(&self) -> i8 {
        for i in (0..self.sub_chunks.len()).rev() {
            if !self.sub_chunks[i].is_all_air() {
                return self.min_sub_chunk_y.wrapping_add(i as i8);
            }
        }
        self.min_sub_chunk_y
    }

    pub fn sub_chunks(&self) -> &[SubChunk] {
        &self.sub_chunks
    }

    pub fn min_sub_chunk_y(&self) -> i8 {
        self.min_sub_chunk_y
    }

    pub fn block_entities(&self) -> &[BlockEntity] {
        &self.block_entities
    }

    pub fn sub_chunk_count(&self) -> usize {
        self.sub_chunks.len()
    }

    pub fn serialize(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        for (i, sc) in self.sub_chunks.iter().enumerate() {
            buf.extend(sc.serialize_network(self.min_sub_chunk_y.wrapping_add(i as i8)))
        }

        for sc in &self.sub_chunks {
            buf.extend(sc.serialize_biomes())
        }

        buf.push(0u8); // border blocks
        buf.extend(self.serialize_block_entities(None));

        buf
    }
}
