use crate::chunk::Chunk;
use crate::generator::dimension::Generator;
use crate::generator::error::PhaseError;
use crate::generator::phase::{Phase, PhaseContext};
use glam::IVec2;

pub struct VoidGenerator {
    pub biome: i32,
    pub air_id: i32,
    pub min_sub_chunk_y: i8,
    pub sub_chunk_count: usize,
}

impl Generator for VoidGenerator {
    type Terminal = VoidPhase;
}

pub struct VoidPhase;

impl Phase<VoidGenerator> for VoidPhase {
    type Output = Chunk;

    fn run(generator: &VoidGenerator, cell: IVec2, _inputs: &mut PhaseContext<VoidGenerator>) -> Result<Self::Output, PhaseError> {
        Ok(Chunk::new(cell.x, cell.y, generator.min_sub_chunk_y, generator.sub_chunk_count, generator.air_id, generator.biome))
    }
}
