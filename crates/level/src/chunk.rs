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

    // TODO: remove this later it's stupid
    pub fn highest_non_air_sub_chunk_y_relative(&self) -> i8 {
        for i in (0..self.sub_chunks.len()).rev() {
            if !self.sub_chunks[i].is_all_air() {
                return self.min_sub_chunk_y.wrapping_add(i as i8);
            }
        }
        self.min_sub_chunk_y
    }

    pub fn highest_non_air_sub_chunk_y(&self) -> i8 {
        for (i, sub_chunk) in self.sub_chunks.iter().enumerate().rev() {
            if !sub_chunk.is_all_air() {
                return (i + 1) as i8;
            }
        }
        0
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

#[cfg(test)]
mod tests {
    use super::*;

    const AIR: i32 = -7;
    const PLAINS: i32 = 1;

    fn overworld() -> Chunk {
        Chunk::new(3, -2, -4, 24, AIR, PLAINS)
    }

    #[test]
    fn a_new_chunk_is_clean_and_all_air() {
        let chunk = overworld();
        assert_eq!((chunk.x, chunk.z), (3, -2));
        assert_eq!(chunk.sub_chunk_count(), 24);
        assert_eq!(chunk.min_sub_chunk_y(), -4);
        assert!(!chunk.is_dirty());
        assert_eq!(chunk.highest_non_air_sub_chunk_y(), 0);
        assert_eq!(chunk.highest_non_air_sub_chunk_y_relative(), -4);
    }

    #[test]
    fn blocks_are_set_and_read_at_the_lowest_and_highest_y() {
        let mut chunk = overworld();
        assert!(chunk.set_block(0, -64, 0, 0, 1));
        assert!(chunk.set_block(15, 319, 15, 0, 2));
        assert_eq!(chunk.get_block(0, -64, 0, 0), Some(1));
        assert_eq!(chunk.get_block(15, 319, 15, 0), Some(2));
        assert_eq!(chunk.get_block(0, -63, 0, 0), Some(AIR));
        assert_eq!(chunk.get_block(15, 318, 15, 0), Some(AIR));
    }

    #[test]
    fn y_outside_the_height_range_is_rejected() {
        let mut chunk = overworld();
        assert_eq!(chunk.get_block(0, -65, 0, 0), None);
        assert_eq!(chunk.get_block(0, 320, 0, 0), None);
        assert!(!chunk.set_block(0, -65, 0, 0, 1));
        assert!(!chunk.set_block(0, 320, 0, 0, 1));
        assert!(!chunk.is_dirty());
    }

    #[test]
    fn blocks_on_either_side_of_a_sub_chunk_boundary_land_in_different_sub_chunks() {
        let mut chunk = overworld();
        for (y, sub_y, local) in [(-1, -1i8, 15u8), (0, 0, 0), (15, 0, 15), (16, 1, 0), (-16, -1, 0), (-17, -2, 15)] {
            assert!(chunk.set_block(4, y, 5, 0, 9), "y {y}");
            assert_eq!(chunk.get_sub_chunk(sub_y).unwrap().get(4, local, 5, 0), 9, "y {y}");
            assert_eq!(chunk.get_block(4, y, 5, 0), Some(9));
        }
        assert_eq!(chunk.get_block(4, -2, 5, 0), Some(AIR));
        assert_eq!(chunk.get_block(4, 1, 5, 0), Some(AIR));
    }

    #[test]
    fn set_block_reports_changes_and_only_marks_the_chunk_dirty_for_real_ones() {
        let mut chunk = overworld();
        assert!(!chunk.set_block(1, 1, 1, 0, AIR));
        assert!(!chunk.is_dirty());
        assert!(chunk.set_block(1, 1, 1, 0, 4));
        assert!(chunk.is_dirty());
        chunk.mark_saved();
        assert!(!chunk.set_block(1, 1, 1, 0, 4));
        assert!(!chunk.is_dirty());
        chunk.mark_dirty();
        assert!(chunk.is_dirty());
    }

    #[test]
    fn layers_hold_separate_blocks() {
        let mut chunk = overworld();
        chunk.set_block(2, 70, 2, 1, 6);
        assert_eq!(chunk.get_block(2, 70, 2, 0), Some(AIR));
        assert_eq!(chunk.get_block(2, 70, 2, 1), Some(6));
    }

    #[test]
    fn biomes_are_set_and_read_across_boundaries() {
        let mut chunk = overworld();
        assert_eq!(chunk.get_biome(0, -64, 0), Some(PLAINS));
        for (y, biome) in [(-64, 10), (-1, 11), (0, 12), (15, 13), (16, 14), (319, 15)] {
            assert!(chunk.set_biome(7, y, 8, biome));
            assert_eq!(chunk.get_biome(7, y, 8), Some(biome), "y {y}");
        }
        assert_eq!(chunk.get_biome(7, 17, 8), Some(PLAINS));
        assert_eq!(chunk.get_biome(8, 0, 8), Some(PLAINS));
    }

    #[test]
    fn biomes_outside_the_height_range_are_rejected() {
        let mut chunk = overworld();
        assert!(!chunk.set_biome(0, 320, 0, 5));
        assert!(!chunk.set_biome(0, -65, 0, 5));
        assert_eq!(chunk.get_biome(0, 320, 0), None);
        assert_eq!(chunk.get_biome(0, -65, 0), None);
    }

    #[test]
    fn a_chunk_that_starts_at_zero_has_no_negative_sub_chunks() {
        let mut chunk = Chunk::new(0, 0, 0, 8, AIR, PLAINS);
        assert!(chunk.get_sub_chunk(-1).is_none());
        assert!(chunk.get_sub_chunk(7).is_some());
        assert!(chunk.get_sub_chunk(8).is_none());
        assert!(!chunk.set_block(0, -1, 0, 0, 1));
        assert!(chunk.set_block(0, 127, 0, 0, 1));
    }

    #[test]
    fn highest_non_air_sub_chunk_tracks_built_sub_chunks_and_edits() {
        let mut blocks = [AIR; 4096];
        blocks[0] = 1;
        let sub_chunks = (0..24).map(|i| if i == 5 { SubChunk::from_blocks(&blocks, AIR, PLAINS) } else { SubChunk::new(AIR, PLAINS) }).collect();
        let mut chunk = Chunk::from_sub_chunks(0, 0, -4, sub_chunks, Vec::new());
        assert_eq!(chunk.highest_non_air_sub_chunk_y_relative(), 1);
        assert_eq!(chunk.highest_non_air_sub_chunk_y(), 6);
        chunk.set_block(0, 16, 0, 0, AIR);
        assert_eq!(chunk.highest_non_air_sub_chunk_y_relative(), -4);
        assert_eq!(chunk.highest_non_air_sub_chunk_y(), 0);
    }

    #[test]
    fn highest_non_air_sub_chunk_sees_a_block_placed_in_an_empty_chunk() {
        let mut chunk = overworld();
        chunk.set_block(0, 20, 0, 0, 1);
        assert_eq!(chunk.highest_non_air_sub_chunk_y_relative(), 1);
    }

    #[test]
    fn mutable_sub_chunk_access_marks_the_chunk_dirty_only_when_it_exists() {
        let mut chunk = overworld();
        assert!(chunk.get_sub_chunk_mut(30).is_none());
        assert!(!chunk.is_dirty());
        assert!(chunk.get_sub_chunk_mut(0).is_some());
        assert!(chunk.is_dirty());
    }

    #[test]
    fn serialisation_writes_every_sub_chunk_then_biomes_then_the_border_byte() {
        let chunk = Chunk::new(0, 0, 0, 2, AIR, PLAINS);
        let bytes = chunk.serialize();
        let sub_chunk = chunk.get_sub_chunk(0).unwrap();
        let mut expected = sub_chunk.serialize_network(0);
        expected.extend(chunk.get_sub_chunk(1).unwrap().serialize_network(1));
        expected.extend(sub_chunk.serialize_biomes());
        expected.extend(sub_chunk.serialize_biomes());
        expected.push(0);
        assert_eq!(bytes, expected);
    }
}
