use crate::level::chunk::Chunk;
use crate::level::generator::WorldGenerator;
use crate::registry::block_registry::BlockRegistry;

pub struct RandomGenerator {
    pub biome: i32,
}

impl WorldGenerator for RandomGenerator {
    fn generate(&self, registry: &BlockRegistry, _x: i32, _z: i32, chunk: &mut Chunk) {
        for lx in 0u8..16 {
            for lz in 0u8..16 {
                chunk.set_block(lx, 0, lz, 0, registry.get_random());
            }
        }
    }
}
