use crate::level::chunk::Chunk;
use crate::level::generator::dimension::Generator;
use crate::level::generator::phase::{Phase, PhaseInputs};
use crate::level::generator::pos::ChunkPos;

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

    fn run(generator: &VoidGenerator, cell: ChunkPos, _inputs: &PhaseInputs<VoidGenerator>) -> Self::Output {
        Chunk::new(cell.x, cell.z, generator.min_sub_chunk_y, generator.sub_chunk_count, generator.air_id, generator.biome)
    }
}
