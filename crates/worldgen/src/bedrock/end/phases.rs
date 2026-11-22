use std::sync::Arc;

use chorus_level::chunk::Chunk;
use chorus_level::generator::error::PhaseError;
use chorus_level::generator::phase::{Dependency, Phase, PhaseContext, SAME_CELL};

use super::cities::{self, City};
use super::{EndChunk, EndGenerator, EndWrites};
use crate::shared::CHUNK_NEIGHBORHOOD;
use crate::shared::decoration_class::{self, Decorates, decorations_around, neighborhood, on_decorations};
use crate::shared::structure::BoundingBox;
use glam::IVec2;

const REACH_SIDE: usize = (cities::REACH * 2 + 1) as usize;
const REACH_CELLS: usize = REACH_SIDE * REACH_SIDE;
const REACH_OFFSETS: [(i32, i32); REACH_CELLS] = reach_offsets();

const fn reach_offsets() -> [(i32, i32); REACH_CELLS] {
    let mut offsets = [(0, 0); REACH_CELLS];
    let mut x = -cities::REACH;
    let mut index = 0;
    while x <= cities::REACH {
        let mut z = -cities::REACH;
        while z <= cities::REACH {
            offsets[index] = (x, z);
            index += 1;
            z += 1;
        }
        x += 1;
    }
    offsets
}

fn city_candidate(cell: IVec2) -> bool {
    super::generator::may_start_city(cell.x, cell.y)
}

pub struct TerrainPhase;

impl Phase<EndGenerator> for TerrainPhase {
    type Output = Arc<EndChunk>;

    const RETAIN: usize = 512;

    fn run(generator: &EndGenerator, cell: IVec2, _inputs: &mut PhaseContext<EndGenerator>) -> Result<Arc<EndChunk>, PhaseError> {
        let mut chunk = generator.fill_terrain(cell.x, cell.y);
        chunk.compact();
        Ok(Arc::new(chunk))
    }
}

pub struct StructureStartPhase;

impl Phase<EndGenerator> for StructureStartPhase {
    type Output = Option<Arc<City>>;

    const RETAIN: usize = 1024;

    fn run(generator: &EndGenerator, cell: IVec2, _inputs: &mut PhaseContext<EndGenerator>) -> Result<Option<Arc<City>>, PhaseError> {
        Ok(generator.city_at((cell.x, cell.y)))
    }
}

pub struct StructurePhase;

impl Phase<EndGenerator> for StructurePhase {
    type Output = Arc<EndChunk>;

    const RETAIN: usize = 512;

    fn dependencies() -> Vec<Dependency<EndGenerator>> {
        vec![
            Dependency::on::<TerrainPhase>(SAME_CELL),
            Dependency::on::<StructureStartPhase>(&REACH_OFFSETS).if_dependency_position(city_candidate),
        ]
    }

    fn run(generator: &EndGenerator, cell: IVec2, context: &mut PhaseContext<EndGenerator>) -> Result<Arc<EndChunk>, PhaseError> {
        let terrain = context.get::<TerrainPhase>(cell)?;
        let column = BoundingBox::chunk(cell.x, cell.y, generator.definition);
        let mut cities: Vec<Arc<City>> = Vec::new();
        for start in generator.city_start_cells((cell.x, cell.y)) {
            if let Some(city) = context.get::<StructureStartPhase>(IVec2::new(start.0, start.1))?.as_ref()
                && city.bounds.intersects(&column)
            {
                cities.push(city.clone());
            }
        }
        let mut chunk = generator.loaded_chunk((cell.x, cell.y), &terrain, &cities);
        chunk.compact();
        Ok(Arc::new(chunk))
    }
}

pub type DecoratePhase<const CLASS: u8> = decoration_class::DecoratePhase<EndGenerator, CLASS>;

impl Decorates for EndGenerator {
    type Decoration = EndWrites;

    const RETAIN: usize = 512;

    fn inputs() -> Vec<Dependency<EndGenerator>> {
        vec![Dependency::on::<StructurePhase>(CHUNK_NEIGHBORHOOD), Dependency::on::<TerrainPhase>(CHUNK_NEIGHBORHOOD)]
    }

    fn decorate(&self, cell: IVec2, context: &PhaseContext<EndGenerator>, earlier: &[((i32, i32), &EndWrites)]) -> Result<EndWrites, PhaseError> {
        let loaded = neighborhood::<EndGenerator, StructurePhase>(context, cell)?;
        let terrain = neighborhood::<EndGenerator, TerrainPhase>(context, cell)?;
        Ok(self.decorate_source((cell.x, cell.y), &loaded, &terrain, earlier))
    }
}

pub struct ChunkPhase;

impl Phase<EndGenerator> for ChunkPhase {
    type Output = Chunk;

    fn dependencies() -> Vec<Dependency<EndGenerator>> {
        let mut dependencies = vec![Dependency::on::<StructurePhase>(SAME_CELL)];
        dependencies.extend(on_decorations(CHUNK_NEIGHBORHOOD));
        dependencies
    }

    fn run(generator: &EndGenerator, cell: IVec2, context: &mut PhaseContext<EndGenerator>) -> Result<Chunk, PhaseError> {
        let loaded = context.get::<StructurePhase>(cell)?;
        let around = decorations_around::<EndGenerator, 9>(context, cell, CHUNK_NEIGHBORHOOD)?;
        let dense = generator.assemble((cell.x, cell.y), &loaded, &around);
        Ok(generator.dense_chunk(cell.x, cell.y, &dense))
    }
}
