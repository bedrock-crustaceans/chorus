use std::sync::Arc;

use glam::IVec3;

use super::super::{Edition, OverworldGenerator};
use super::blocks::{AIR, BlockId, Blocks};
use super::proto::{ProtoChunk, QuartBiomes};
use super::region::{BlockView, Changes};
use super::survival::Support;
use super::terrain::{CornerColumns, NoiseChunk};
use crate::block::block_id::{COCOA, GLOW_LICHEN, SCULK_VEIN, VINE};
use crate::error::phase::PhaseError;
use crate::level::chunk::Chunk;
use crate::level::generator::dimension::Generator;
use crate::level::generator::phase::{Phase, PhaseInputs, Requirement, SAME_CELL};
use crate::level::generator::pos::ChunkPos;

impl<E: Edition> Generator for OverworldGenerator<E> {
    type Terminal = ChunkPhase;
}

pub struct BiomePhase;

impl<E: Edition> Phase<OverworldGenerator<E>> for BiomePhase {
    type Output = QuartBiomes;

    const RETAIN: usize = 1024;

    fn run(generator: &OverworldGenerator<E>, cell: ChunkPos, _inputs: &mut PhaseInputs<OverworldGenerator<E>>) -> Result<QuartBiomes, PhaseError> {
        Ok(generator.quart_biomes(cell.x, cell.z))
    }
}

pub struct CornerPhase;

impl<E: Edition> Phase<OverworldGenerator<E>> for CornerPhase {
    type Output = CornerColumns;

    const RETAIN: usize = 1024;

    fn run(generator: &OverworldGenerator<E>, cell: ChunkPos, _inputs: &mut PhaseInputs<OverworldGenerator<E>>) -> Result<CornerColumns, PhaseError> {
        Ok(generator.terrain.corner_columns(cell.x, cell.z))
    }
}

const SHARED_CORNERS: &[(i32, i32)] = &[(0, 0), (1, 0), (0, 1), (1, 1)];

pub struct NoisePhase;

impl<E: Edition> Phase<OverworldGenerator<E>> for NoisePhase {
    type Output = (ProtoChunk, NoiseChunk, E::Aquifer);

    fn requires() -> Vec<Requirement<OverworldGenerator<E>>> {
        vec![BiomePhase::at(NEIGHBORHOOD), CornerPhase::at(SHARED_CORNERS)]
    }

    fn run(generator: &OverworldGenerator<E>, cell: ChunkPos, inputs: &mut PhaseInputs<OverworldGenerator<E>>) -> Result<(ProtoChunk, NoiseChunk, E::Aquifer), PhaseError> {
        let around = NEIGHBORHOOD
            .iter()
            .map(|&(dx, dz)| inputs.get::<BiomePhase>(ChunkPos::new(cell.x + dx, cell.z + dz)))
            .collect::<Result<Vec<_>, _>>()?;
        let corners = SHARED_CORNERS
            .iter()
            .map(|&(dx, dz)| inputs.get::<CornerPhase>(ChunkPos::new(cell.x + dx, cell.z + dz)))
            .collect::<Result<Vec<_>, _>>()?;
        let noise = NoiseChunk::assemble(std::array::from_fn(|i| &*corners[i]));
        Ok(generator.fill_noise(cell.x, cell.z, std::array::from_fn(|i| &*around[i]), noise))
    }
}

pub struct SurfacePhase;

impl<E: Edition> Phase<OverworldGenerator<E>> for SurfacePhase {
    type Output = (ProtoChunk, NoiseChunk, E::Aquifer);

    fn requires() -> Vec<Requirement<OverworldGenerator<E>>> {
        vec![NoisePhase::at(SAME_CELL)]
    }

    fn run(generator: &OverworldGenerator<E>, cell: ChunkPos, inputs: &mut PhaseInputs<OverworldGenerator<E>>) -> Result<(ProtoChunk, NoiseChunk, E::Aquifer), PhaseError> {
        let (mut chunk, noise, aquifer) = inputs.take::<NoisePhase>(cell)?;
        generator.build_surface(&noise, &mut chunk);
        Ok((chunk, noise, aquifer))
    }
}

const NEIGHBORHOOD: &[(i32, i32)] = &[(-1, -1), (-1, 0), (-1, 1), (0, -1), (0, 0), (0, 1), (1, -1), (1, 0), (1, 1)];
const EARLIER_OVERLAPPING: &[(i32, i32)] = &[(-2, -2), (-1, -2), (0, -2), (1, -2), (2, -2), (-2, -1), (-1, -1), (0, -1), (1, -1), (2, -1), (-2, 0), (-1, 0)];
const OWNERS: &[(i32, i32)] = &[(-1, -1), (0, -1), (1, -1), (-1, 0), (0, 0), (1, 0), (-1, 1), (0, 1), (1, 1)];

pub struct CarverPhase;

impl<E: Edition> Phase<OverworldGenerator<E>> for CarverPhase {
    type Output = ProtoChunk;

    const RETAIN: usize = 1024;

    fn requires() -> Vec<Requirement<OverworldGenerator<E>>> {
        vec![SurfacePhase::at(SAME_CELL)]
    }

    fn run(generator: &OverworldGenerator<E>, cell: ChunkPos, inputs: &mut PhaseInputs<OverworldGenerator<E>>) -> Result<ProtoChunk, PhaseError> {
        let (mut chunk, noise, mut aquifer) = inputs.take::<SurfacePhase>(cell)?;
        generator.apply_carvers(&noise, &mut aquifer, &mut chunk);
        Ok(chunk)
    }
}

pub struct FeaturePhase;

impl<E: Edition> Phase<OverworldGenerator<E>> for FeaturePhase {
    type Output = Changes;

    const RETAIN: usize = 1024;

    fn requires() -> Vec<Requirement<OverworldGenerator<E>>> {
        vec![CarverPhase::at(NEIGHBORHOOD), FeaturePhase::at(EARLIER_OVERLAPPING).max_hops(1)]
    }

    fn run(generator: &OverworldGenerator<E>, cell: ChunkPos, inputs: &mut PhaseInputs<OverworldGenerator<E>>) -> Result<Changes, PhaseError> {
        let earlier: Vec<_> = EARLIER_OVERLAPPING
            .iter()
            .filter_map(|&(dx, dz)| inputs.try_get::<FeaturePhase>(ChunkPos::new(cell.x + dx, cell.z + dz)))
            .collect();
        let mut chunks = Vec::with_capacity(9);
        for &(dx, dz) in NEIGHBORHOOD {
            let (x, z) = (cell.x + dx, cell.z + dz);
            let baseline = inputs.get::<CarverPhase>(ChunkPos::new(x, z))?;
            let mut chunk = (*baseline).clone();
            for changes in &earlier {
                changes.overlay_onto(x, z, &baseline, &mut chunk);
            }
            chunks.push(chunk);
        }
        let chunks: [ProtoChunk; 9] = chunks.try_into().unwrap_or_else(|_| unreachable!());
        Ok(generator.decorate(cell.x, cell.z, chunks))
    }
}

pub struct ColumnPhase;

impl<E: Edition> Phase<OverworldGenerator<E>> for ColumnPhase {
    type Output = ProtoChunk;

    const RETAIN: usize = 1024;

    fn requires() -> Vec<Requirement<OverworldGenerator<E>>> {
        vec![CarverPhase::at(SAME_CELL), FeaturePhase::at(OWNERS)]
    }

    fn run(generator: &OverworldGenerator<E>, cell: ChunkPos, inputs: &mut PhaseInputs<OverworldGenerator<E>>) -> Result<ProtoChunk, PhaseError> {
        let baseline = inputs.get::<CarverPhase>(cell)?;
        let mut column = (*baseline).clone();
        for &(dx, dz) in OWNERS {
            let owner = inputs.get::<FeaturePhase>(ChunkPos::new(cell.x + dx, cell.z + dz))?;
            owner.overlay_onto(cell.x, cell.z, &baseline, &mut column);
            for &(x, y, z, block) in owner.placed(cell.x, cell.z) {
                if needs_support(&generator.blocks, block) {
                    column.mark_settle(x, y, z);
                }
            }
        }
        Ok(column)
    }
}

fn needs_support(blocks: &Blocks, block: BlockId) -> bool {
    let entry = blocks.entry(block);
    entry.support != Support::Anywhere || entry.upper.is_some() || matches!(entry.name, VINE | COCOA | GLOW_LICHEN | SCULK_VEIN)
}

struct Finished<'a> {
    blocks: &'a Blocks,
    cell: ChunkPos,
    center: &'a ProtoChunk,
    around: &'a [Arc<ProtoChunk>],
}

impl BlockView for Finished<'_> {
    fn blocks(&self) -> &Blocks {
        self.blocks
    }

    fn get(&self, pos: IVec3) -> BlockId {
        let (dx, dz) = ((pos.x >> 4) - self.cell.x, (pos.z >> 4) - self.cell.z);
        let Some(index) = NEIGHBORHOOD.iter().position(|&offset| offset == (dx, dz)) else { return AIR };
        let chunk = if (dx, dz) == (0, 0) { self.center } else { &self.around[index] };
        chunk.get(pos.x & 15, pos.y, pos.z & 15)
    }
}

pub struct PostProcessPhase;

impl<E: Edition> Phase<OverworldGenerator<E>> for PostProcessPhase {
    type Output = ProtoChunk;

    fn requires() -> Vec<Requirement<OverworldGenerator<E>>> {
        vec![ColumnPhase::at(NEIGHBORHOOD)]
    }

    fn run(generator: &OverworldGenerator<E>, cell: ChunkPos, inputs: &mut PhaseInputs<OverworldGenerator<E>>) -> Result<ProtoChunk, PhaseError> {
        let around = NEIGHBORHOOD
            .iter()
            .map(|&(dx, dz)| inputs.get::<ColumnPhase>(ChunkPos::new(cell.x + dx, cell.z + dz)))
            .collect::<Result<Vec<_>, _>>()?;
        let mut chunk = (*inputs.get::<ColumnPhase>(cell)?).clone();
        let shapes = generator.edition.shapes(generator.water);
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
        Ok(chunk)
    }
}

pub struct ChunkPhase;

impl<E: Edition> Phase<OverworldGenerator<E>> for ChunkPhase {
    type Output = Chunk;

    fn requires() -> Vec<Requirement<OverworldGenerator<E>>> {
        vec![PostProcessPhase::at(SAME_CELL)]
    }

    fn run(generator: &OverworldGenerator<E>, cell: ChunkPos, inputs: &mut PhaseInputs<OverworldGenerator<E>>) -> Result<Chunk, PhaseError> {
        Ok(inputs.take::<PostProcessPhase>(cell)?.into_chunk(&generator.blocks))
    }
}
