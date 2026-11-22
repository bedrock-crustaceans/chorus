use crate::chunk::Chunk;
use crate::generator::dimension::Generator;
use crate::generator::error::PhaseError;
use crate::generator::phase::{Phase, PhaseContext};
use glam::IVec2;

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

    fn run(generator: &FlatGenerator, cell: IVec2, _inputs: &mut PhaseContext<FlatGenerator>) -> Result<Chunk, PhaseError> {
        let mut chunk = Chunk::new(cell.x, cell.y, generator.min_sub_chunk_y, generator.sub_chunk_count, generator.air_id, generator.biome);

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

        Ok(chunk)
    }
}
