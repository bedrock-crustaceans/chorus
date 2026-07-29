use crate::level::chunk::Chunk;
use crate::level::generator::WorldGenerator;
use crate::registry::block_registry::BlockRegistry;

pub struct VoidGenerator {
    pub biome: i32,
}

impl WorldGenerator for VoidGenerator {
    fn generate(&self, _registry: &BlockRegistry, _x: i32, _z: i32, _chunk: &mut Chunk) {}
}
