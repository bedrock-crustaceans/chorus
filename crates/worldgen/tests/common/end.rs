use super::harness::GraphHarness;
use chorus_level::generator::dimension::Dimension;
use chorus_worldgen::bedrock::end::EndGenerator;
use chorus_worldgen::bedrock::end::phases::StructureStartPhase;
use chorus_worldgen::shared::structure::BoundingBox;
use std::time::{Duration, Instant};

pub type Graph = GraphHarness<EndGenerator>;

pub fn graph(seed: i64) -> Graph {
    GraphHarness::new(EndGenerator::new(seed, &super::registry()))
}

pub trait Stages {
    fn city_starts(&self, chunk_x: i32, chunk_z: i32) -> usize;
}

impl Stages for Graph {
    fn city_starts(&self, chunk_x: i32, chunk_z: i32) -> usize {
        let column = BoundingBox::chunk(chunk_x, chunk_z, chorus_level::dimension_definition::DimensionDefinition::END);
        self.city_start_cells((chunk_x, chunk_z))
            .into_iter()
            .filter_map(|start| self.fetch::<StructureStartPhase>(start).as_ref().clone())
            .filter(|city| city.bounds.intersects(&column))
            .count()
    }
}

pub fn load(dimension: &mut Dimension, positions: &[(i32, i32)]) {
    dimension.request_chunks(positions);
    let start = Instant::now();
    while positions.iter().any(|&(x, z)| dimension.get_chunk(x, z).is_none()) {
        dimension.tick();
        assert!(start.elapsed() < Duration::from_secs(300), "the chunks never generated");
    }
}
