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
    fn generate(&self, registry: &BlockRegistry, x: i32, z: i32, min_sub_chunk_y: i8, max_sub_chunk_y: i8) -> Chunk {
        let air_id = registry.get_block_id("minecraft:air").unwrap_or(0);
        let count = (max_sub_chunk_y as i32 - min_sub_chunk_y as i32 + 1) as usize;
        let mut chunk = Chunk::new(x, z, min_sub_chunk_y, count, air_id, self.biome);

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
        chunk
    }
}
