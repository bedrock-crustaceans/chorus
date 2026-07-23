use crate::level::chunk::Chunk;
use crate::level::generator::WorldGenerator;
use crate::registry::block_registry::BlockRegistry;

pub struct RandomGenerator {
    pub biome: i32,
}

impl WorldGenerator for RandomGenerator {
    fn generate(&self, registry: &BlockRegistry, x: i32, z: i32, min_sub_chunk_y: i8, max_sub_chunk_y: i8) -> Chunk {
        let air_id = registry.get_block_id("minecraft:air").unwrap_or(0);
        let count = (max_sub_chunk_y as i32 - min_sub_chunk_y as i32 + 1) as usize;
        let mut chunk = Chunk::new(x, z, min_sub_chunk_y, count, air_id, self.biome);

        for lx in 0u8..16 {
            for lz in 0u8..16 {
                chunk.set_block(lx, 0, lz, 0, registry.get_random());
            }
        }

        chunk
    }
}
