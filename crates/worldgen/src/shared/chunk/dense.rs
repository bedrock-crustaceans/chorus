use chorus_level::chunk::Chunk;
use chorus_level::dimension_definition::DimensionDefinition;
use chorus_level::sub_chunk::SubChunk;

use crate::shared::blocks::{AIR, BlockId, Blocks};

const COLUMN_WIDTH: usize = 16;
const SECTION_HEIGHT: usize = 16;
const SECTION_CELLS: usize = 4096;

pub fn from_dense(position: (i32, i32), definition: DimensionDefinition, cells: &[BlockId], blocks: &Blocks, biome: impl Fn(i32, i32) -> i32) -> Chunk {
    let height = definition.height as usize;
    let air = blocks.runtime(AIR);
    let sections = height / SECTION_HEIGHT;
    let mut layer = [air; SECTION_CELLS];
    let mut biomes = [0i32; SECTION_CELLS];
    let mut sub_chunks = Vec::with_capacity(sections);
    let column_biomes: [i32; COLUMN_WIDTH * COLUMN_WIDTH] = std::array::from_fn(|column| biome((column / COLUMN_WIDTH) as i32, (column % COLUMN_WIDTH) as i32));
    for section in 0..sections {
        for x in 0..COLUMN_WIDTH {
            for z in 0..COLUMN_WIDTH {
                let column_biome = column_biomes[x * COLUMN_WIDTH + z];
                for y in 0..SECTION_HEIGHT {
                    let index = SubChunk::index(x as u8, y as u8, z as u8);
                    let dense = (x * COLUMN_WIDTH + z) * height + section * SECTION_HEIGHT + y;
                    layer[index] = blocks.runtime(cells[dense]);
                    biomes[index] = column_biome;
                }
            }
        }
        sub_chunks.push(SubChunk::from_layers(&layer, None, &biomes, air));
    }
    Chunk::from_sub_chunks(position.0, position.1, definition.min_sub_chunk_y(), sub_chunks, Vec::new())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chorus_block::block_id::STONE;
    use chorus_block::block_registry::BlockRegistry;
    use chorus_block::r#impl::DEFINITIONS;

    use crate::shared::blocks::BlockTable;

    fn registry() -> BlockRegistry {
        let mut registry = BlockRegistry::new();
        registry.register_all(DEFINITIONS.iter().copied());
        registry
    }

    #[test]
    fn every_dense_cell_lands_on_its_absolute_coordinates() {
        let registry = registry();
        let mut table = BlockTable::new(&registry);
        let stone = table.name(STONE);
        let blocks = table.finish();
        let mut cells = vec![AIR; COLUMN_WIDTH * COLUMN_WIDTH * 128];
        cells[(3 * 16 + 7) * 128 + 9] = stone;
        cells[(15 * 16) * 128 + 127] = stone;

        let chunk = from_dense((2, -3), DimensionDefinition::NETHER, &cells, &blocks, |x, z| x * 16 + z);

        assert_eq!((chunk.x, chunk.z), (2, -3));
        assert_eq!(chunk.min_sub_chunk_y(), 0);
        assert_eq!(chunk.sub_chunk_count(), 8);
        assert_eq!(chunk.get_block(3, 9, 7, 0), Some(blocks.runtime(stone)));
        assert_eq!(chunk.get_block(15, 127, 0, 0), Some(blocks.runtime(stone)));
        assert_eq!(chunk.get_block(0, 0, 0, 0), Some(blocks.runtime(AIR)));
        assert!(chunk.block_entities().is_empty());
    }
}
