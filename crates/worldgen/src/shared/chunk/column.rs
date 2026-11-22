use super::store::BlockStore;
use crate::shared::blocks::{AIR, BlockId};
use chorus_level::dimension_definition::DimensionDefinition;

const WIDTH: i32 = 16;
const COLUMNS: usize = (WIDTH * WIDTH) as usize;

#[derive(Clone)]
pub struct ColumnChunk {
    store: BlockStore,
    definition: DimensionDefinition,
}

impl ColumnChunk {
    pub fn empty(definition: DimensionDefinition) -> Self {
        Self {
            store: BlockStore::new(definition.min_y, definition.height, AIR),
            definition,
        }
    }

    pub fn definition(&self) -> DimensionDefinition {
        self.definition
    }

    pub fn min_y(&self) -> i32 {
        self.definition.min_y
    }

    pub fn max_y(&self) -> i32 {
        self.definition.max_y()
    }

    pub fn contains_y(&self, y: i32) -> bool {
        (self.definition.min_y..self.definition.top()).contains(&y)
    }

    pub fn cells(&self) -> usize {
        COLUMNS * self.definition.height as usize
    }

    pub fn index(&self, x: i32, y: i32, z: i32) -> usize {
        ((x * WIDTH + z) as usize) * self.definition.height as usize + (y - self.definition.min_y) as usize
    }

    pub fn get(&self, x: i32, y: i32, z: i32) -> BlockId {
        self.store.get(x, y, z)
    }

    pub fn set(&mut self, x: i32, y: i32, z: i32, block: BlockId) {
        self.store.set(x, y, z, block);
    }

    pub fn get_at(&self, index: usize) -> BlockId {
        let (x, y, z) = self.coordinates(index);
        self.store.get(x, y, z)
    }

    pub fn set_at(&mut self, index: usize, block: BlockId) {
        let (x, y, z) = self.coordinates(index);
        self.store.set(x, y, z, block);
    }

    pub fn cells_vec(&self) -> Vec<BlockId> {
        let mut cells = Vec::with_capacity(self.cells());
        for x in 0..WIDTH {
            for z in 0..WIDTH {
                cells.extend((self.definition.min_y..self.definition.top()).map(|y| self.store.get(x, y, z)));
            }
        }
        cells
    }

    pub fn compact(&mut self) {
        self.store.compact();
    }

    fn coordinates(&self, index: usize) -> (i32, i32, i32) {
        let height = self.definition.height as usize;
        let column = index / height;
        ((column / WIDTH as usize) as i32, self.definition.min_y + (index % height) as i32, (column % WIDTH as usize) as i32)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const NETHER: DimensionDefinition = DimensionDefinition::NETHER;

    #[test]
    fn indexed_access_matches_coordinate_access() {
        let mut chunk = ColumnChunk::empty(NETHER);
        chunk.set(3, 77, 12, 9);
        assert_eq!(chunk.get_at(chunk.index(3, 77, 12)), 9);
        chunk.set_at(chunk.index(15, 0, 15), 4);
        assert_eq!(chunk.get(15, 0, 15), 4);

        let mut chunk = ColumnChunk::empty(NETHER);
        chunk.set(1, 2, 3, 7);
        let cells = chunk.cells_vec();
        assert_eq!(cells.len(), chunk.cells());
        assert_eq!(cells[chunk.index(1, 2, 3)], 7);
        assert_eq!(cells.iter().filter(|&&block| block != AIR).count(), 1);
    }

    #[test]
    fn a_shifted_range_indexes_from_its_floor() {
        let mut chunk = ColumnChunk::empty(DimensionDefinition { min_y: -32, height: 96 });
        chunk.set(2, -32, 5, 6);
        chunk.set(2, 63, 5, 8);
        assert_eq!(chunk.get_at(chunk.index(2, -32, 5)), 6);
        assert_eq!(chunk.get_at(chunk.index(2, 63, 5)), 8);
        assert!(chunk.contains_y(-32) && chunk.contains_y(63) && !chunk.contains_y(64) && !chunk.contains_y(-33));
        assert_eq!(chunk.cells_vec()[chunk.index(2, 63, 5)], 8);
    }
}
