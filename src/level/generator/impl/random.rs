use std::sync::Arc;

use crate::level::chunk::Chunk;
use crate::level::generator::dimension::Generator;
use crate::level::generator::phase::{Phase, PhaseInputs, PhaseValue};
use crate::level::generator::pos::ChunkPos;
use rand::prelude::IndexedRandom;
use rand::rng;

pub struct RandomGenerator {
    pub biome: i32,
    pub air_id: i32,
    pub block_ids: Vec<i32>,
    pub min_sub_chunk_y: i8,
    pub sub_chunk_count: usize,
}

impl Generator for RandomGenerator {
    type Terminal = RandomPhase;
    type Value = Arc<Chunk>;
}

pub struct RandomPhase;

impl PhaseValue<RandomGenerator> for RandomPhase {
    fn wrap(output: Arc<Self::Output>) -> Arc<Chunk> {
        output
    }

    fn unwrap(value: &Arc<Chunk>) -> Option<Arc<Self::Output>> {
        Some(value.clone())
    }
}

impl Phase<RandomGenerator> for RandomPhase {
    type Output = Chunk;

    fn run(generator: &RandomGenerator, cell: ChunkPos, _inputs: &PhaseInputs<RandomGenerator>) -> Self::Output {
        let mut chunk = Chunk::new(cell.x, cell.z, generator.min_sub_chunk_y, generator.sub_chunk_count, generator.air_id, generator.biome);

        for lx in 0u8..16 {
            for lz in 0u8..16 {
                chunk.set_block(lx, 0, lz, 0, *generator.block_ids.choose(&mut rng()).expect("no hash found"));
            }
        }

        chunk
    }
}
