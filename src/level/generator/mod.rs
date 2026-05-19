pub mod flat;

use crate::level::chunk::Chunk;
use crate::registry::block_registry::BlockRegistry;

pub trait WorldGenerator: Send + Sync {
    fn generate(&self, registry: &BlockRegistry, x: i32, z: i32, min_sub_chunk_y: i8, max_sub_chunk_y: i8) -> Chunk;
}
