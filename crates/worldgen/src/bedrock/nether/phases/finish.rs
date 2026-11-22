use super::super::decorate::SourceDecoration;
use super::super::{NetherChunk, NetherGenerator};
use super::REACH_TWO;
use super::structured::{StructureJoinPhase, Structured, reach_two_index};
use crate::shared::CHUNK_NEIGHBORHOOD;
use crate::shared::decoration_class::{decorations_around, on_decorations};
use crate::shared::decoration_window::{assemble, sources_near};
use chorus_level::chunk::Chunk;
use chorus_level::generator::error::PhaseError;
use chorus_level::generator::phase::{Dependency, Phase, PhaseContext, SAME_CELL};
use glam::IVec2;
use std::sync::Arc;

impl NetherGenerator {
    pub(in super::super) fn fluid_updates(&self, target: (i32, i32), around: &[Arc<SourceDecoration>; 25]) -> Vec<chorus_level::fluid::PendingUpdate> {
        let mut updates = Vec::new();
        for source in sources_near(target) {
            for tick in &around[reach_two_index(source.0 - target.0, source.1 - target.1)].routed {
                if tick.target == target {
                    updates.push(chorus_level::fluid::PendingUpdate {
                        pos: glam::IVec3::new(tick.pos[0], tick.pos[1], tick.pos[2]),
                        lava: tick.lava,
                        delay: tick.delay,
                    });
                }
            }
        }
        updates
    }
    pub(in super::super) fn assemble(&self, cell: (i32, i32), structured: &Structured, around: &[Arc<SourceDecoration>; 9]) -> NetherChunk {
        assemble(cell, (*structured.chunk).clone(), |index| &around[index].writes)
    }
}

pub struct FluidTicksPhase;

impl Phase<NetherGenerator> for FluidTicksPhase {
    type Output = Vec<chorus_level::fluid::PendingUpdate>;

    const RETAIN: usize = 0;

    fn dependencies() -> Vec<Dependency<NetherGenerator>> {
        let mut dependencies = vec![Dependency::on::<StructureJoinPhase>(CHUNK_NEIGHBORHOOD)];
        dependencies.extend(on_decorations(REACH_TWO));
        dependencies
    }

    fn run(generator: &NetherGenerator, cell: IVec2, context: &mut PhaseContext<NetherGenerator>) -> Result<Vec<chorus_level::fluid::PendingUpdate>, PhaseError> {
        let around = decorations_around::<NetherGenerator, 25>(context, cell, REACH_TWO)?;
        Ok(generator.fluid_updates((cell.x, cell.y), &around))
    }
}

pub struct ChunkPhase;

impl Phase<NetherGenerator> for ChunkPhase {
    type Output = Chunk;

    fn dependencies() -> Vec<Dependency<NetherGenerator>> {
        let mut dependencies = vec![Dependency::on::<StructureJoinPhase>(SAME_CELL)];
        dependencies.extend(on_decorations(CHUNK_NEIGHBORHOOD));
        dependencies.push(Dependency::on::<FluidTicksPhase>(SAME_CELL));
        dependencies
    }

    fn run(generator: &NetherGenerator, cell: IVec2, context: &mut PhaseContext<NetherGenerator>) -> Result<Chunk, PhaseError> {
        let structured = (*context.get::<StructureJoinPhase>(cell)?).clone();
        let around = decorations_around::<NetherGenerator, 9>(context, cell, CHUNK_NEIGHBORHOOD)?;
        let updates = context.take::<FluidTicksPhase>(cell)?;
        let dense = generator.assemble((cell.x, cell.y), &structured, &around);
        let mut chunk = generator.dense_chunk(cell.x, cell.y, &dense);
        chunk.set_fluid_updates(updates);
        Ok(chunk)
    }
}
