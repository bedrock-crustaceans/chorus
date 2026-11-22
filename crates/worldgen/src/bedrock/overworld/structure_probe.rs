use chorus_level::dimension_definition::DimensionDefinition;
use std::sync::Arc;

use rustc_hash::FxHashMap;

use super::OverworldGenerator;
use super::phases::{CarverPhase, HeightsPhase, RawTerrainPhase, StartsPhase};
use crate::shared::blocks::BlockId;
use crate::shared::chunk::proto::ProtoChunk;
use crate::shared::chunk::region::Region;
use crate::shared::rng::mersenne::MersenneTwister;
use crate::shared::structure::{ColumnHeights, MissingInput, NO_SURFACE, Probed, StartProbe, Starts};
use chorus_level::generator::engine::Delivery;
use chorus_level::generator::error::{PhaseError, PhaseFailure};
use chorus_level::generator::phase::{PhaseContext, PhaseId};
use glam::{IVec2, IVec3};

impl From<MissingInput> for PhaseError {
    fn from(missing: MissingInput) -> Self {
        PhaseError::custom(missing)
    }
}

const COLUMN_AREA: i32 = 16;
const AROUND_SIDE: i32 = 3;
const AROUND: usize = 9;

pub trait Supply: Sync {
    fn heights(&self, cell: (i32, i32)) -> Option<Arc<ColumnHeights>>;
    fn chunk(&self, cell: (i32, i32)) -> Option<Arc<ProtoChunk>>;
    fn raw_chunk(&self, cell: (i32, i32)) -> Option<Arc<ProtoChunk>>;
    fn starts(&self, cell: (i32, i32)) -> Option<Arc<Starts>>;
}

impl Supply for PhaseContext<OverworldGenerator> {
    fn heights(&self, cell: (i32, i32)) -> Option<Arc<ColumnHeights>> {
        self.try_get::<HeightsPhase>(IVec2::new(cell.0, cell.1))
    }

    fn chunk(&self, cell: (i32, i32)) -> Option<Arc<ProtoChunk>> {
        self.try_get::<CarverPhase>(IVec2::new(cell.0, cell.1))
    }

    fn raw_chunk(&self, cell: (i32, i32)) -> Option<Arc<ProtoChunk>> {
        self.try_get::<RawTerrainPhase>(IVec2::new(cell.0, cell.1))
    }

    fn starts(&self, cell: (i32, i32)) -> Option<Arc<Starts>> {
        self.try_get::<StartsPhase>(IVec2::new(cell.0, cell.1))
    }
}

#[derive(Default)]
pub(crate) struct Arrived {
    heights: FxHashMap<(i32, i32), Arc<ColumnHeights>>,
    chunks: FxHashMap<(i32, i32), Arc<ProtoChunk>>,
    raw_chunks: FxHashMap<(i32, i32), Arc<ProtoChunk>>,
    starts: FxHashMap<(i32, i32), Arc<Starts>>,
}

impl Arrived {
    pub fn absorb(&mut self, arrivals: Vec<Delivery>) -> Result<(), Arc<PhaseFailure>> {
        for delivery in arrivals {
            let cell = (delivery.cell.x, delivery.cell.y);
            let output = delivery.outcome?;
            if delivery.phase == PhaseId::of::<HeightsPhase>() {
                self.heights.insert(cell, output.downcast::<ColumnHeights>().expect("heights phase output"));
            } else if delivery.phase == PhaseId::of::<CarverPhase>() {
                self.chunks.insert(cell, output.downcast::<ProtoChunk>().expect("carver phase output"));
            } else if delivery.phase == PhaseId::of::<RawTerrainPhase>() {
                self.raw_chunks.insert(cell, output.downcast::<ProtoChunk>().expect("raw terrain phase output"));
            } else if delivery.phase == PhaseId::of::<StartsPhase>() {
                self.starts.insert(cell, output.downcast::<Starts>().expect("starts phase output"));
            }
        }
        Ok(())
    }
}

impl Supply for Arrived {
    fn heights(&self, cell: (i32, i32)) -> Option<Arc<ColumnHeights>> {
        self.heights.get(&cell).cloned()
    }

    fn chunk(&self, cell: (i32, i32)) -> Option<Arc<ProtoChunk>> {
        self.chunks.get(&cell).cloned()
    }

    fn raw_chunk(&self, cell: (i32, i32)) -> Option<Arc<ProtoChunk>> {
        self.raw_chunks.get(&cell).cloned()
    }

    fn starts(&self, cell: (i32, i32)) -> Option<Arc<Starts>> {
        self.starts.get(&cell).cloned()
    }
}

pub struct BedrockProbe<'a> {
    pub generator: &'a OverworldGenerator,
    supply: &'a dyn Supply,
    raw: bool,
}

impl<'a> BedrockProbe<'a> {
    pub fn reading(generator: &'a OverworldGenerator, supply: &'a dyn Supply) -> Self {
        Self { generator, supply, raw: false }
    }

    pub fn for_starts(generator: &'a OverworldGenerator, supply: &'a dyn Supply) -> Self {
        Self { generator, supply, raw: true }
    }

    fn terrain_of(&self, chunk: (i32, i32)) -> Option<Arc<ProtoChunk>> {
        if self.raw { self.supply.raw_chunk(chunk) } else { self.supply.chunk(chunk) }
    }

    fn carved(&self, chunk: (i32, i32)) -> Probed<Arc<ProtoChunk>> {
        self.terrain_of(chunk).ok_or(MissingInput { what: "carved terrain", cell: chunk })
    }
}

impl StartProbe for BedrockProbe<'_> {
    fn preliminary_surface(&self, (x, z): (i32, i32)) -> i32 {
        self.surface_at_quart((x * 4, z * 4))
    }

    fn surface_at_quart(&self, (x, z): (i32, i32)) -> i32 {
        let surface = self.generator.terrain.preliminary_surface(x, z);
        if surface == super::terrain::SURFACE_NOT_FOUND { NO_SURFACE } else { surface }
    }

    fn biome(&self, x: i32, y: i32, z: i32) -> i32 {
        self.generator.biomes.point(&self.generator.terrain, x, y, z).bedrock_id()
    }

    fn sea_level(&self) -> i32 {
        self.generator.terrain.sea_level()
    }

    fn definition(&self) -> DimensionDefinition {
        self.generator.terrain.definition()
    }

    fn block(&self, x: i32, y: i32, z: i32) -> Probed<Option<BlockId>> {
        let chunk = self.carved((x >> 4, z >> 4))?;
        Ok(Some(chunk.get(x & 15, y, z & 15)))
    }

    fn adjusted_block(&self, x: i32, y: i32, z: i32) -> Probed<Option<BlockId>> {
        let chunk = self.carved((x >> 4, z >> 4))?;
        Ok(Some(chunk.get(x & 15, y, z & 15)))
    }

    fn chunk(&self, chunk_x: i32, chunk_z: i32) -> Probed<Option<ProtoChunk>> {
        let chunk = self.carved((chunk_x, chunk_z))?;
        Ok(Some((*chunk).clone()))
    }

    fn run_feature(&self, chunk: &mut ProtoChunk, id: &str, origin: (i32, i32, i32), random: &mut MersenneTwister) -> Probed<bool> {
        let decoration = &self.generator.decoration;
        if !decoration.knows(id) {
            return Ok(false);
        }
        let centre: &ProtoChunk = chunk;
        let around: [ProtoChunk; AROUND] = std::array::from_fn(|slot| {
            let (dx, dz) = (slot as i32 / AROUND_SIDE - 1, slot as i32 % AROUND_SIDE - 1);
            if (dx, dz) == (0, 0) { centre.clone() } else { centre.empty_beside(dx, dz) }
        });
        let (chunk_x, chunk_z) = (chunk.min_x().div_euclid(COLUMN_AREA), chunk.min_z().div_euclid(COLUMN_AREA));
        let surface = |quart_x: i32, quart_z: i32| self.generator.terrain.preliminary_surface(quart_x, quart_z);
        let mut region = Region::new(chunk_x, chunk_z, self.generator.blocks(), around)
            .with_preliminary_surface(&surface)
            .with_sea_level(self.generator.terrain.sea_level());
        decoration.place_named(&mut region, id, random, IVec3::new(origin.0, origin.1, origin.2))?;
        let changes = region.into_changes();
        changes[AROUND / 2].apply_to(chunk);
        for (slot, changed) in changes.iter().enumerate().filter(|&(slot, _)| slot != AROUND / 2) {
            let (base_x, base_z) = ((chunk_x + slot as i32 / AROUND_SIDE - 1) * COLUMN_AREA, (chunk_z + slot as i32 % AROUND_SIDE - 1) * COLUMN_AREA);
            chunk.add_structure_spills(changed.changes().map(|(x, y, z, block)| (base_x + x as i32, y as i32, base_z + z as i32, block)));
        }
        Ok(true)
    }

    fn column_heights(&self, chunk: (i32, i32)) -> Probed<Arc<ColumnHeights>> {
        self.supply.heights(chunk).ok_or(MissingInput { what: "column heights", cell: chunk })
    }

    fn supplied_heights(&self, chunk: (i32, i32)) -> Option<Arc<ColumnHeights>> {
        self.supply.heights(chunk)
    }

    fn supplied_chunk(&self, chunk: (i32, i32)) -> Option<Arc<ProtoChunk>> {
        self.terrain_of(chunk)
    }

    fn starts_at(&self, cell: (i32, i32)) -> Option<Arc<Starts>> {
        self.supply.starts(cell)
    }
}
