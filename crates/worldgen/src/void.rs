use chorus_level::chunk::Chunk;
use chorus_level::generator::dimension::Generator;
use chorus_level::generator::error::PhaseError;
use chorus_level::generator::phase::{Phase, PhaseInputs};
use chorus_level::generator::pos::ChunkPos;

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

    fn run(generator: &VoidGenerator, cell: ChunkPos, _inputs: &mut PhaseInputs<VoidGenerator>) -> Result<Self::Output, PhaseError> {
        Ok(Chunk::new(cell.x, cell.z, generator.min_sub_chunk_y, generator.sub_chunk_count, generator.air_id, generator.biome))
    }
}
