use crate::level::chunk::Chunk;
use crate::level::generator::dimension::Generator;
use crate::level::generator::phase::{Phase, PhaseInputs};
use crate::level::generator::pos::ChunkPos;

pub struct FlatLayer {
    pub block_id: i32,
    pub height: u32,
}

pub struct FlatGenerator {
    pub layers: Vec<FlatLayer>,
    pub biome: i32,
    pub air_id: i32,
    pub min_sub_chunk_y: i8,
    pub sub_chunk_count: usize,
}

impl Generator for FlatGenerator {
    type Terminal = FlatPhase;
}

pub struct FlatPhase;

impl Phase<FlatGenerator> for FlatPhase {
    type Output = Chunk;

    fn run(generator: &FlatGenerator, cell: ChunkPos, _inputs: &PhaseInputs<FlatGenerator>) -> Chunk {
        let mut chunk = Chunk::new(cell.x, cell.z, generator.min_sub_chunk_y, generator.sub_chunk_count, generator.air_id, generator.biome);

        let mut y = 0i32;
        for layer in &generator.layers {
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
