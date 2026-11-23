use super::super::{NetherChunk, NetherGenerator};
use super::structured::{StructureJoinPhase, Structured};
use super::terrain::BiomePhase;
use crate::shared::CHUNK_NEIGHBORHOOD;
use crate::shared::decoration_class::{decorations_around, on_decorations};
use crate::shared::decoration_window::{SourceChanges, assemble, offset_slot, sources_near};
use chorus_level::chunk::Chunk;
use chorus_level::generator::error::PhaseError;
use chorus_level::generator::phase::{Dependency, Phase, PhaseContext, SAME_CELL};
use glam::IVec2;
use std::sync::Arc;

impl NetherGenerator {
    pub(in super::super) fn fluid_updates(&self, target: (i32, i32), around: &[Arc<SourceChanges>; 9]) -> Vec<chorus_level::fluid::PendingUpdate> {
        let mut updates = Vec::new();
        for source in sources_near(target) {
            let mailed = &around[offset_slot(target, source)][offset_slot(source, target)];
            updates.extend(mailed.routed_ticks.iter().map(|tick| chorus_level::fluid::PendingUpdate {
                pos: tick.pos,
                lava: tick.lava,
                delay: tick.delay,
            }));
        }
        updates
    }
    pub(in super::super) fn assemble(&self, cell: (i32, i32), structured: &Structured, around: &[Arc<SourceChanges>; 9]) -> NetherChunk {
        assemble(cell, (*structured.chunk).clone(), |index| &around[index])
    }
}

pub struct ChunkPhase;

impl Phase<NetherGenerator> for ChunkPhase {
    type Output = Chunk;

    fn dependencies() -> Vec<Dependency<NetherGenerator>> {
        let mut dependencies = vec![Dependency::on::<StructureJoinPhase>(SAME_CELL), Dependency::on::<BiomePhase>(SAME_CELL)];
        dependencies.extend(on_decorations(CHUNK_NEIGHBORHOOD));
        dependencies
    }

    fn run(generator: &NetherGenerator, cell: IVec2, context: &mut PhaseContext<NetherGenerator>) -> Result<Chunk, PhaseError> {
        let structured = (*context.get::<StructureJoinPhase>(cell)?).clone();
        let around = decorations_around::<NetherGenerator, 9>(context, cell, CHUNK_NEIGHBORHOOD)?;
        let updates = generator.fluid_updates((cell.x, cell.y), &around);
        let dense = generator.assemble((cell.x, cell.y), &structured, &around);
        let biomes = context.get::<BiomePhase>(cell)?;
        let mut chunk = generator.dense_chunk(cell.x, cell.y, &dense, &biomes);
        chunk.set_fluid_updates(updates);
        generator.active_fluid_ticks(&mut chunk);
        Ok(chunk)
    }
}
