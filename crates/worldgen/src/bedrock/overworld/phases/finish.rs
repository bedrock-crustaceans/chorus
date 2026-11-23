use super::super::OverworldGenerator;
use super::chain;
use super::feature::{FeaturePhase0, FeaturePhase1, FeaturePhase2, FeaturePhase3, OWNERS, merge_phases};
use super::structure::{structured, structured_baseline};
use crate::shared::CHUNK_NEIGHBORHOOD;
use crate::shared::blocks::{AIR, BlockId, Blocks};
use crate::shared::chunk::proto::{ProtoChunk, RoutedTick};
use crate::shared::chunk::region::{BlockView, Changes};
use crate::shared::decoration_class::is_class;
use crate::shared::survival::Support;
use chorus_block::block_id::{COCOA, GLOW_LICHEN, SCULK_VEIN, VINE};
use chorus_level::chunk::Chunk;
use chorus_level::generator::error::PhaseError;
use chorus_level::generator::phase::{Dependency, Phase, PhaseContext, SAME_CELL};
use glam::{IVec2, IVec3};
use std::sync::Arc;

const CENTRE: usize = 4;

pub struct ColumnPhase;

impl Phase<OverworldGenerator> for ColumnPhase {
    type Output = ProtoChunk;

    const RETAIN: usize = 1024;

    fn dependencies() -> Vec<Dependency<OverworldGenerator>> {
        vec![
            structured(SAME_CELL),
            chain::<FeaturePhase0>(OWNERS).with_mail(SAME_CELL).if_dependency(is_class::<0>),
            chain::<FeaturePhase1>(OWNERS).with_mail(SAME_CELL).if_dependency(is_class::<1>),
            chain::<FeaturePhase2>(OWNERS).with_mail(SAME_CELL).if_dependency(is_class::<2>),
            chain::<FeaturePhase3>(OWNERS).with_mail(SAME_CELL).if_dependency(is_class::<3>),
        ]
    }

    fn run(generator: &OverworldGenerator, cell: IVec2, context: &mut PhaseContext<OverworldGenerator>) -> Result<ProtoChunk, PhaseError> {
        let baseline = structured_baseline(context, cell)?;
        let mut column = (*baseline).clone();
        {
            let mut edit = column.edit();
            let mut mail: Vec<(IVec2, &Changes)> = Vec::new();
            mail.extend(context.mail::<FeaturePhase0>(cell));
            mail.extend(context.mail::<FeaturePhase1>(cell));
            mail.extend(context.mail::<FeaturePhase2>(cell));
            mail.extend(context.mail::<FeaturePhase3>(cell));
            let mut routed: Vec<RoutedTick> = Vec::new();
            let mut consumed: Vec<RoutedTick> = Vec::new();
            let mut pending: Vec<RoutedTick> = Vec::new();
            merge_phases(&mail, &generator.blocks, &mut edit);
            for (source, changes) in mail {
                for (x, y, z, block) in changes.changes() {
                    if needs_support(&generator.blocks, block) {
                        edit.mark_settle(x, y, z);
                    }
                }
                routed.extend_from_slice(&changes.routed_ticks);
                if source == cell {
                    consumed.extend_from_slice(&changes.consumed_ticks);
                    pending.extend_from_slice(&changes.pending_fluid);
                }
            }
            if !routed.is_empty() {
                routed.retain(|tick| !consumed.contains(tick));
                edit.set_routed_ticks(routed);
            }
            edit.set_pending_fluid(pending);
        }
        Ok(column)
    }
}

fn needs_support(blocks: &Blocks, block: BlockId) -> bool {
    let entry = blocks.entry(block);
    entry.support != Support::Anywhere || entry.upper.is_some() || matches!(entry.name.as_ref(), VINE | COCOA | GLOW_LICHEN | SCULK_VEIN)
}

struct Finished<'a> {
    blocks: &'a Blocks,
    cell: IVec2,
    center: &'a ProtoChunk,
    around: &'a [Arc<ProtoChunk>],
}

impl BlockView for Finished<'_> {
    fn blocks(&self) -> &Blocks {
        self.blocks
    }

    fn get(&self, pos: IVec3) -> BlockId {
        let (dx, dz) = ((pos.x >> 4) - self.cell.x, (pos.z >> 4) - self.cell.y);
        let Some(index) = CHUNK_NEIGHBORHOOD.iter().position(|&offset| offset == (dx, dz)) else {
            return AIR;
        };
        let chunk = if (dx, dz) == (0, 0) { self.center } else { &self.around[index] };
        chunk.get(pos.x & 15, pos.y, pos.z & 15)
    }
}

pub struct PostProcessPhase;

impl Phase<OverworldGenerator> for PostProcessPhase {
    type Output = ProtoChunk;

    fn dependencies() -> Vec<Dependency<OverworldGenerator>> {
        vec![Dependency::on::<ColumnPhase>(CHUNK_NEIGHBORHOOD)]
    }

    fn run(generator: &OverworldGenerator, cell: IVec2, context: &mut PhaseContext<OverworldGenerator>) -> Result<ProtoChunk, PhaseError> {
        let around = CHUNK_NEIGHBORHOOD
            .iter()
            .map(|&(dx, dz)| context.get::<ColumnPhase>(cell + IVec2::new(dx, dz)))
            .collect::<Result<Vec<_>, _>>()?;
        let mut owned = (*around[CENTRE]).clone();
        {
            let mut pending: Vec<RoutedTick> = owned
                .routed_ticks()
                .iter()
                .map(|tick| RoutedTick {
                    pos: tick.pos,
                    lava: tick.lava,
                    delay: if tick.lava { tick.delay } else { 0 },
                })
                .collect();
            pending.extend_from_slice(owned.pending_fluid());
            owned.set_pending_fluid(pending);
        }
        let mut chunk = owned.edit();
        let shapes = generator.shapes();
        let marks = chunk.post_process().to_vec();
        for &(x, y, z) in &marks {
            let (x, y, z) = (x as i32, y as i32, z as i32);
            let state = chunk.get(x, y, z);
            if generator.blocks.entry(state).liquid {
                continue;
            }
            let pos = IVec3::new(chunk.min_x() + x, y, chunk.min_z() + z);
            let view = Finished {
                blocks: &generator.blocks,
                cell,
                center: &chunk,
                around: &around,
            };
            let updated = shapes.update_from_neighbours(&view, pos);
            if updated != state {
                chunk.set(x, y, z, updated);
            }
        }
        let mut unsettled = chunk.take_settle();
        loop {
            let before = unsettled.len();
            let mut index = 0;
            while index < unsettled.len() {
                let (x, y, z) = unsettled[index];
                let (x, y, z) = (x as i32, y as i32, z as i32);
                let state = chunk.get(x, y, z);
                if !needs_support(&generator.blocks, state) {
                    unsettled.swap_remove(index);
                    continue;
                }
                let pos = IVec3::new(chunk.min_x() + x, y, chunk.min_z() + z);
                let view = Finished {
                    blocks: &generator.blocks,
                    cell,
                    center: &chunk,
                    around: &around,
                };
                let updated = shapes.update_from_neighbours(&view, pos);
                if generator.blocks.is_air(updated) || updated == generator.water {
                    chunk.set(x, y, z, updated);
                    unsettled.swap_remove(index);
                } else {
                    index += 1;
                }
            }
            if unsettled.len() == before {
                break;
            }
        }
        drop(chunk);
        Ok(owned)
    }
}

pub struct ChunkPhase;

impl Phase<OverworldGenerator> for ChunkPhase {
    type Output = Chunk;

    fn dependencies() -> Vec<Dependency<OverworldGenerator>> {
        vec![chain::<PostProcessPhase>(SAME_CELL)]
    }

    fn run(generator: &OverworldGenerator, cell: IVec2, context: &mut PhaseContext<OverworldGenerator>) -> Result<Chunk, PhaseError> {
        let mut chunk = context.take::<PostProcessPhase>(cell)?.into_chunk(&generator.blocks);
        generator.active_fluid_ticks(&mut chunk);
        Ok(chunk)
    }
}
