use crate::block::block_id;
use crate::level::chunk::Chunk;
use crate::level::generator::WorldGenerator;
use crate::registry::block_registry::BlockRegistry;

pub struct VoidGenerator {
    pub biome: i32,
}

impl WorldGenerator for VoidGenerator {
    fn generate(&self, registry: &BlockRegistry, x: i32, z: i32, min_sub_chunk_y: i8, max_sub_chunk_y: i8) -> Chunk {
        let air_id = registry.get_block_id(block_id::AIR).unwrap_or(0);
        let count = (max_sub_chunk_y as i32 - min_sub_chunk_y as i32 + 1) as usize;

        Chunk::new(x, z, min_sub_chunk_y, count, air_id, self.biome)
    }
}
