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

fn entity_column(data: &nbtx::Value) -> Option<(i32, i32)> {
    let tag = data.as_compound()?;
    let coordinate = |name: &str| match tag.get(name.as_bytes()) {
        Some(nbtx::Value::Int(value)) => Some(*value),
        _ => None,
    };
    Some((coordinate("x")?, coordinate("z")?))
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
        for entity in self.block_entities.iter().filter(|entity| sub_y.is_none_or(|sub_y| (entity.y >> 4) == sub_y as i32)) {
            buf.extend(nbtx::to_varint_bytes(&entity.data).expect("block entity nbt serializes"));
        }
        buf
    }

    pub fn set_biome(&mut self, x: u8, y: i32, z: u8, biome: i32) -> bool {
        match self.sub_chunk_for_y_mut(y) {
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

    fn sub_chunk_index_for_y(&self, y: i32) -> Option<usize> {
        usize::try_from((y >> 4) - self.min_sub_chunk_y as i32).ok()
    }

    fn sub_chunk_for_y_mut(&mut self, y: i32) -> Option<&mut SubChunk> {
        let index = self.sub_chunk_index_for_y(y)?;
        let sub_chunk = self.sub_chunks.get_mut(index);
        self.dirty |= sub_chunk.is_some();
        sub_chunk
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
        let local_y = (y & 0xF) as u8;
        Some(self.sub_chunks.get(self.sub_chunk_index_for_y(y)?)?.get(x, local_y, z, layer))
    }

    pub fn get_biome(&self, x: u8, y: i32, z: u8) -> Option<i32> {
        Some(self.sub_chunks.get(self.sub_chunk_index_for_y(y)?)?.get_biome(x, (y & 0xF) as u8, z))
    }

    pub fn set_block(&mut self, x: u8, y: i32, z: u8, layer: usize, block_id: i32) -> bool {
        let Some(index) = self.sub_chunk_index_for_y(y) else {
            return false;
        };
        let changed = self.sub_chunks.get_mut(index).is_some_and(|sub_chunk| sub_chunk.set(x, (y & 0xF) as u8, z, layer, block_id));
        self.dirty |= changed;
        if changed && layer == 0 {
            self.remove_block_entity(self.x * 16 + i32::from(x), y, self.z * 16 + i32::from(z));
        }
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

    pub fn block_entity_at(&self, x: i32, y: i32, z: i32) -> Option<&nbtx::Value> {
        self.block_entities
            .iter()
            .find(|entity| entity.y == y && entity_column(&entity.data) == Some((x, z)))
            .map(|entity| &entity.data)
    }

    pub fn set_block_entity(&mut self, x: i32, y: i32, z: i32, data: nbtx::Value) {
        match self.block_entities.iter_mut().find(|entity| entity.y == y && entity_column(&entity.data) == Some((x, z))) {
            Some(entity) => entity.data = data,
            None => self.block_entities.push(BlockEntity { y, data }),
        }
        self.dirty = true;
    }

    pub fn remove_block_entity(&mut self, x: i32, y: i32, z: i32) -> bool {
        let before = self.block_entities.len();
        self.block_entities.retain(|entity| !(entity.y == y && entity_column(&entity.data) == Some((x, z))));
        let removed = self.block_entities.len() != before;
        self.dirty |= removed;
        removed
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

    fn chunk() -> Chunk {
        Chunk::new(0, 0, -4, 24, 0, 0)
    }

    #[test]
    fn in_range_blocks_round_trip() {
        let mut chunk = chunk();
        assert!(chunk.set_block(1, -64, 2, 0, 7));
        assert_eq!(chunk.get_block(1, -64, 2, 0), Some(7));
        assert!(chunk.set_block(1, 319, 2, 0, 9));
        assert_eq!(chunk.get_block(1, 319, 2, 0), Some(9));
    }

    #[test]
    fn out_of_range_y_does_not_alias() {
        let mut chunk = chunk();
        assert_eq!(chunk.get_block(0, 4096, 0, 0), None);
        assert_eq!(chunk.get_block(0, -65, 0, 0), None);
        assert_eq!(chunk.get_block(0, 320, 0, 0), None);
        assert!(!chunk.set_block(0, 4096, 0, 0, 5));
        assert!(!chunk.set_block(0, -4096, 0, 0, 5));
        assert_eq!(chunk.get_block(0, 0, 0, 0), Some(0));
        assert_eq!(chunk.get_biome(0, 4096, 0), None);
        assert!(!chunk.set_biome(0, 4096, 0, 3));
    }
}

#[cfg(test)]
mod block_entity_tests {
    use super::*;

    fn entity(x: i32, y: i32, z: i32, tag: i8) -> nbtx::Value {
        let mut compound = nbtx::Compound::new();
        compound.insert("x".into(), nbtx::Value::Int(x));
        compound.insert("y".into(), nbtx::Value::Int(y));
        compound.insert("z".into(), nbtx::Value::Int(z));
        compound.insert("marker".into(), nbtx::Value::Byte(tag));
        nbtx::Value::Compound(compound)
    }

    fn marker(chunk: &Chunk, x: i32, y: i32, z: i32) -> Option<i8> {
        match chunk.block_entity_at(x, y, z)?.as_compound()?.get("marker".as_bytes())? {
            nbtx::Value::Byte(value) => Some(*value),
            _ => None,
        }
    }

    fn chunk() -> Chunk {
        Chunk::new(0, 0, -4, 24, 0, 0)
    }

    #[test]
    fn a_block_entity_is_found_where_it_was_put() {
        let mut chunk = chunk();
        assert!(chunk.block_entity_at(3, 64, 5).is_none());
        chunk.set_block_entity(3, 64, 5, entity(3, 64, 5, 1));
        assert_eq!(marker(&chunk, 3, 64, 5), Some(1));
        assert!(chunk.block_entity_at(3, 65, 5).is_none());
        assert!(chunk.block_entity_at(4, 64, 5).is_none());
    }

    #[test]
    fn setting_again_replaces_instead_of_adding() {
        let mut chunk = chunk();
        chunk.set_block_entity(3, 64, 5, entity(3, 64, 5, 1));
        chunk.set_block_entity(3, 64, 5, entity(3, 64, 5, 2));
        assert_eq!(chunk.block_entities().len(), 1);
        assert_eq!(marker(&chunk, 3, 64, 5), Some(2));
    }

    #[test]
    fn two_blocks_in_one_sub_chunk_keep_separate_entities() {
        let mut chunk = chunk();
        chunk.set_block_entity(1, 64, 1, entity(1, 64, 1, 1));
        chunk.set_block_entity(2, 64, 1, entity(2, 64, 1, 2));
        assert_eq!(marker(&chunk, 1, 64, 1), Some(1));
        assert_eq!(marker(&chunk, 2, 64, 1), Some(2));
    }

    #[test]
    fn removing_reports_whether_there_was_one_and_marks_the_chunk_to_save() {
        let mut chunk = chunk();
        assert!(!chunk.remove_block_entity(3, 64, 5));
        assert!(!chunk.is_dirty());
        chunk.set_block_entity(3, 64, 5, entity(3, 64, 5, 1));
        chunk.mark_saved();
        assert!(chunk.remove_block_entity(3, 64, 5));
        assert!(chunk.is_dirty());
        assert!(chunk.block_entity_at(3, 64, 5).is_none());
    }

    #[test]
    fn replacing_the_block_removes_its_block_entity() {
        let mut chunk = Chunk::new(2, -1, -4, 24, 0, 0);
        let (x, y, z) = (2 * 16 + 3, 64, -16 + 5);
        chunk.set_block(3, y, 5, 0, 9);
        chunk.set_block_entity(x, y, z, entity(x, y, z, 1));
        assert!(chunk.block_entity_at(x, y, z).is_some());
        assert!(!chunk.set_block(3, y, 5, 0, 9), "the same block changes nothing");
        assert!(chunk.block_entity_at(x, y, z).is_some());
        assert!(chunk.set_block(3, y, 5, 0, 0));
        assert!(chunk.block_entity_at(x, y, z).is_none());
    }

    #[test]
    fn changing_a_block_entity_marks_the_chunk_to_save() {
        let mut chunk = chunk();
        chunk.set_block_entity(3, 64, 5, entity(3, 64, 5, 1));
        assert!(chunk.is_dirty());
    }
}
