use crate::level::chunk::Chunk;
use crate::level::generator::WorldGenerator;
use crate::registry::block_registry::BlockRegistry;

pub struct FlatLayer {
    pub block_id: i32,
    pub height: u32,
}

pub struct FlatGenerator {
    pub layers: Vec<FlatLayer>,
    pub biome: i32,
}

impl WorldGenerator for FlatGenerator {
    fn generate(&self, _registry: &BlockRegistry, _x: i32, _z: i32, chunk: &mut Chunk) {
        let mut y = 0i32;
        for layer in &self.layers {
            for _ in 0..layer.height {
                for lx in 0u8..16 {
                    for lz in 0u8..16 {
                        chunk.set_block(lx, y, lz, 0, layer.block_id);
                    }
                }
                y += 1;
            }
        }
    }
}
