use super::super::OverworldGenerator;
use super::super::aquifer::{HINT_RADIUS, SurfaceHints, SurfaceWindow};
use super::chain;
use crate::shared::CHUNK_NEIGHBORHOOD;
use crate::shared::cell_phase::{CellOutput, CellPhase};
use crate::shared::chunk::proto::{ProtoChunk, QuartBiomes};
use crate::shared::footprint;
use crate::shared::terrain::{CornerColumns, NoiseChunk};
use chorus_level::generator::error::PhaseError;
use chorus_level::generator::phase::{Dependency, Phase, PhaseContext, Request, SAME_CELL};
use glam::IVec2;

const SHARED_CORNERS: &[(i32, i32)] = &[(0, 0), (1, 0), (0, 1), (1, 1)];

const HINT_WINDOW: [(i32, i32); footprint::cells(HINT_RADIUS)] = footprint::square(HINT_RADIUS);

fn surface_window(cell: IVec2, context: &PhaseContext<OverworldGenerator>) -> Result<SurfaceWindow, PhaseError> {
    let hints = HINT_WINDOW
        .iter()
        .map(|&(dx, dz)| context.get::<SurfaceHintsPhase>(cell + IVec2::new(dx, dz)))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(SurfaceWindow::new(cell.x, cell.y, &hints))
}

pub struct BiomeColumnCells;

impl CellOutput<OverworldGenerator> for BiomeColumnCells {
    type Value = Vec<i32>;

    const NAME: &'static str = "BiomeColumnsPhase";

    const RETAIN: usize = 1024;

    fn compute(generator: &OverworldGenerator, cell: IVec2) -> Vec<i32> {
        generator.biome_columns(cell.x, cell.y)
    }
}

pub type BiomeColumnsPhase = CellPhase<BiomeColumnCells>;

pub struct BiomePhase;

impl Phase<OverworldGenerator> for BiomePhase {
    type Output = QuartBiomes;

    const RETAIN: usize = 1024;

    const NAME: &'static str = "BiomePhase";

    fn dependencies() -> Vec<Dependency<OverworldGenerator>> {
        vec![chain::<BiomeColumnsPhase>(CHUNK_NEIGHBORHOOD)]
    }

    fn run(generator: &OverworldGenerator, cell: IVec2, context: &mut PhaseContext<OverworldGenerator>) -> Result<QuartBiomes, PhaseError> {
        let mut around = Vec::with_capacity(CHUNK_NEIGHBORHOOD.len());
        for &(dx, dz) in CHUNK_NEIGHBORHOOD {
            around.push(((dx, dz), context.get::<BiomeColumnsPhase>(cell + IVec2::new(dx, dz))?));
        }
        Ok(generator.quart_biomes(|dx, dz| around.iter().find(|(offset, _)| *offset == (dx, dz)).map(|(_, columns)| columns.clone()).expect("a neighbouring cell")))
    }
}

pub struct SurfaceHintCells;

impl CellOutput<OverworldGenerator> for SurfaceHintCells {
    type Value = SurfaceHints;

    const NAME: &'static str = "SurfaceHintsPhase";

    const RETAIN: usize = 1024;

    fn compute(generator: &OverworldGenerator, cell: IVec2) -> SurfaceHints {
        generator.surface_hints(cell.x, cell.y)
    }
}

pub type SurfaceHintsPhase = CellPhase<SurfaceHintCells>;

pub struct CornerCells;

impl CellOutput<OverworldGenerator> for CornerCells {
    type Value = CornerColumns;

    const NAME: &'static str = "CornerPhase";

    const RETAIN: usize = 0;

    fn compute(generator: &OverworldGenerator, cell: IVec2) -> CornerColumns {
        generator.corner_columns(cell.x, cell.y)
    }
}

pub type CornerPhase = CellPhase<CornerCells>;

pub struct FillPhase;

impl Phase<OverworldGenerator> for FillPhase {
    type Output = (ProtoChunk, NoiseChunk, super::super::Aquifer);

    fn dependencies() -> Vec<Dependency<OverworldGenerator>> {
        vec![chain::<BiomePhase>(CHUNK_NEIGHBORHOOD), chain::<CornerPhase>(SHARED_CORNERS), chain::<SurfaceHintsPhase>(&HINT_WINDOW)]
    }

    fn run(generator: &OverworldGenerator, cell: IVec2, context: &mut PhaseContext<OverworldGenerator>) -> Result<(ProtoChunk, NoiseChunk, super::super::Aquifer), PhaseError> {
        let around = CHUNK_NEIGHBORHOOD
            .iter()
            .map(|&(dx, dz)| context.get::<BiomePhase>(cell + IVec2::new(dx, dz)))
            .collect::<Result<Vec<_>, _>>()?;
        let corners = SHARED_CORNERS
            .iter()
            .map(|&(dx, dz)| context.get::<CornerPhase>(cell + IVec2::new(dx, dz)))
            .collect::<Result<Vec<_>, _>>()?;
        let noise = NoiseChunk::assemble(std::array::from_fn(|i| &*corners[i]));
        Ok(generator.fill_unadjusted(cell.x, cell.y, std::array::from_fn(|i| &*around[i]), noise, surface_window(cell, context)?))
    }
}

pub struct NoisePhase;

impl Phase<OverworldGenerator> for NoisePhase {
    type Output = (ProtoChunk, NoiseChunk, super::super::Aquifer);

    fn dependencies() -> Vec<Dependency<OverworldGenerator>> {
        vec![
            chain::<BiomePhase>(CHUNK_NEIGHBORHOOD),
            chain::<CornerPhase>(SHARED_CORNERS),
            chain::<SurfaceHintsPhase>(&HINT_WINDOW),
            chain::<FillPhase>(SAME_CELL),
        ]
    }

    fn resolve(generator: &OverworldGenerator, cell: IVec2, context: &PhaseContext<OverworldGenerator>) -> Vec<Request<OverworldGenerator>> {
        generator.adjustment_requests((cell.x, cell.y), context)
    }

    fn run(generator: &OverworldGenerator, cell: IVec2, context: &mut PhaseContext<OverworldGenerator>) -> Result<(ProtoChunk, NoiseChunk, super::super::Aquifer), PhaseError> {
        let adjustments = generator.adjustments_at(cell.x, cell.y, &*context)?;
        if adjustments.is_empty() {
            return context.take::<FillPhase>(cell);
        }
        let around = CHUNK_NEIGHBORHOOD
            .iter()
            .map(|&(dx, dz)| context.get::<BiomePhase>(cell + IVec2::new(dx, dz)))
            .collect::<Result<Vec<_>, _>>()?;
        let corners = SHARED_CORNERS
            .iter()
            .map(|&(dx, dz)| context.get::<CornerPhase>(cell + IVec2::new(dx, dz)))
            .collect::<Result<Vec<_>, _>>()?;
        let noise = NoiseChunk::assemble(std::array::from_fn(|i| &*corners[i]));
        Ok(generator.fill_with(cell.x, cell.y, std::array::from_fn(|i| &*around[i]), noise, surface_window(cell, context)?, &adjustments))
    }
}

pub struct SurfacePhase;

impl Phase<OverworldGenerator> for SurfacePhase {
    type Output = (ProtoChunk, NoiseChunk, super::super::Aquifer);

    fn dependencies() -> Vec<Dependency<OverworldGenerator>> {
        vec![chain::<NoisePhase>(SAME_CELL)]
    }

    fn run(generator: &OverworldGenerator, cell: IVec2, context: &mut PhaseContext<OverworldGenerator>) -> Result<(ProtoChunk, NoiseChunk, super::super::Aquifer), PhaseError> {
        let (mut chunk, noise, aquifer) = context.take::<NoisePhase>(cell)?;
        generator.build_surface(&mut chunk.edit());
        Ok((chunk, noise, aquifer))
    }
}

pub struct CarverPhase;

impl Phase<OverworldGenerator> for CarverPhase {
    type Output = ProtoChunk;

    const RETAIN: usize = 1024;

    fn dependencies() -> Vec<Dependency<OverworldGenerator>> {
        vec![chain::<SurfacePhase>(SAME_CELL)]
    }

    fn run(generator: &OverworldGenerator, cell: IVec2, context: &mut PhaseContext<OverworldGenerator>) -> Result<ProtoChunk, PhaseError> {
        let (mut chunk, _, mut aquifer) = context.take::<SurfacePhase>(cell)?;
        generator.apply_carvers(&mut aquifer, &mut chunk.edit());
        Ok(chunk)
    }
}

pub struct RawTerrainPhase;

impl Phase<OverworldGenerator> for RawTerrainPhase {
    type Output = ProtoChunk;

    const RETAIN: usize = 1024;

    fn dependencies() -> Vec<Dependency<OverworldGenerator>> {
        vec![chain::<FillPhase>(SAME_CELL)]
    }

    fn run(generator: &OverworldGenerator, cell: IVec2, context: &mut PhaseContext<OverworldGenerator>) -> Result<ProtoChunk, PhaseError> {
        let (mut chunk, _, mut aquifer) = context.take::<FillPhase>(cell)?;
        generator.build_surface(&mut chunk.edit());
        generator.apply_carvers(&mut aquifer, &mut chunk.edit());
        Ok(chunk)
    }
}
