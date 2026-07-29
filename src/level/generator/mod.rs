pub mod flat;
pub mod random;
pub mod void;
pub mod legacy;

use crate::level::chunk::Chunk;
use crate::registry::block_registry::BlockRegistry;

pub trait WorldGenerator: Send + Sync {
    fn generate(&self, registry: &BlockRegistry, x: i32, z: i32, chunk: &mut Chunk);
}
