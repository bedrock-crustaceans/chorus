use super::harness::GraphHarness;
use super::seeds::DIMENSIONS_1;
use super::with_ring;
use chorus_block::block_registry::BlockRegistry;
use chorus_level::dimension_type::DimensionType;
use chorus_level::generator::dimension::Dimension;
use chorus_worldgen::bedrock::nether::phases::{CarverPhase, StructureJoinPhase};
use chorus_worldgen::bedrock::nether::{NetherChunk, NetherGenerator};
use std::time::{Duration, Instant};

pub type Graph = GraphHarness<NetherGenerator>;

pub fn graph(seed: i64) -> Graph {
    GraphHarness::new(NetherGenerator::new(seed, &super::registry()))
}

pub trait Stages {
    fn carved_blocks(&self, chunk_x: i32, chunk_z: i32) -> NetherChunk;
    fn structured_blocks(&self, chunk_x: i32, chunk_z: i32) -> NetherChunk;
}

impl Stages for Graph {
    fn carved_blocks(&self, chunk_x: i32, chunk_z: i32) -> NetherChunk {
        (*self.fetch::<CarverPhase>((chunk_x, chunk_z))).clone()
    }

    fn structured_blocks(&self, chunk_x: i32, chunk_z: i32) -> NetherChunk {
        (*self.fetch::<StructureJoinPhase>((chunk_x, chunk_z)).chunk).clone()
    }
}

const DIMENSION_SEED: i64 = DIMENSIONS_1;

pub fn settled(registry: &BlockRegistry, order: &[(i32, i32)]) -> Dimension {
    bevy_tasks::AsyncComputeTaskPool::get_or_init(bevy_tasks::TaskPool::default);
    let mut dimension = Dimension::new(DimensionType::Nether, NetherGenerator::new(DIMENSION_SEED, registry));
    let requested = with_ring(order);
    dimension.request_chunks(&requested);
    let start = Instant::now();
    while requested.iter().any(|&(x, z)| dimension.get_chunk(x, z).is_none()) {
        dimension.tick();
        assert!(start.elapsed() < Duration::from_secs(600), "the chunks never generated");
    }
    dimension.settle_fluids(chorus_level::fluid::SETTLE_TICKS);
    dimension
}
