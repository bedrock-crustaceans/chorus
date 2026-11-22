use super::super::OverworldGenerator;
use super::chain;
use super::terrain::{CarverPhase, FillPhase};
use crate::shared::chunk::proto::ProtoChunk;
use crate::shared::structure::{Starts, column_heights_of};
use chorus_level::generator::error::PhaseError;
use chorus_level::generator::phase::{Dependency, Phase, PhaseContext, Request, SAME_CELL};
use glam::IVec2;
use std::sync::Arc;

pub struct StartsPhase;

impl Phase<OverworldGenerator> for StartsPhase {
    type Output = Starts;

    const RETAIN: usize = 1024;

    fn resolve(generator: &OverworldGenerator, cell: IVec2, context: &PhaseContext<OverworldGenerator>) -> Vec<Request<OverworldGenerator>> {
        generator.start_requests((cell.x, cell.y), context)
    }

    fn run(generator: &OverworldGenerator, cell: IVec2, context: &mut PhaseContext<OverworldGenerator>) -> Result<Starts, PhaseError> {
        Ok(generator.built_starts((cell.x, cell.y), context)?)
    }
}

pub struct HeightsPhase;

impl Phase<OverworldGenerator> for HeightsPhase {
    type Output = crate::shared::structure::ColumnHeights;

    const RETAIN: usize = 1024;

    fn dependencies() -> Vec<Dependency<OverworldGenerator>> {
        vec![chain::<FillPhase>(SAME_CELL)]
    }

    fn run(generator: &OverworldGenerator, cell: IVec2, context: &mut PhaseContext<OverworldGenerator>) -> Result<Self::Output, PhaseError> {
        let filled = context.get::<FillPhase>(cell)?;
        let mut chunk = filled.0.clone();
        generator.build_surface(&mut chunk.edit());
        Ok(column_heights_of(&chunk, generator.blocks()))
    }
}

pub struct StructurePhase;

impl Phase<OverworldGenerator> for StructurePhase {
    type Output = Arc<ProtoChunk>;

    const RETAIN: usize = 1024;

    fn dependencies() -> Vec<Dependency<OverworldGenerator>> {
        vec![chain::<CarverPhase>(SAME_CELL)]
    }

    fn resolve(generator: &OverworldGenerator, cell: IVec2, context: &PhaseContext<OverworldGenerator>) -> Vec<Request<OverworldGenerator>> {
        let Ok(carved) = context.get::<CarverPhase>(cell) else { return Vec::new() };
        generator.structure_requests((cell.x, cell.y), &carved, context)
    }

    fn run(generator: &OverworldGenerator, cell: IVec2, context: &mut PhaseContext<OverworldGenerator>) -> Result<Arc<ProtoChunk>, PhaseError> {
        let carved = context.get::<CarverPhase>(cell)?;
        let Some(reaching) = generator.structure_reaching((cell.x, cell.y), context)? else {
            return Ok(carved);
        };
        let mut chunk = (*carved).clone();
        generator.structure_postprocess(&reaching, &mut chunk.edit(), context)?;
        Ok(Arc::new(chunk))
    }
}

pub(super) fn structured_baseline(context: &PhaseContext<OverworldGenerator>, cell: IVec2) -> Result<Arc<ProtoChunk>, PhaseError> {
    Ok((*context.get::<StructurePhase>(cell)?).clone())
}

pub(super) fn structured(offsets: &'static [(i32, i32)]) -> Dependency<OverworldGenerator> {
    chain::<StructurePhase>(offsets)
}
