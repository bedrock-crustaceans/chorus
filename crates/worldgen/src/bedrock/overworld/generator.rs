use std::sync::Arc;

use glam::{IVec2, IVec3};
use tracing::warn;

use super::aquifer::{Aquifer, SurfaceHints, SurfaceWindow};
use super::biomes::BiomeSampler;
use super::carver::Carvers;
use super::columns::BufferNoises;
use super::decoration::Decoration;
use super::decoration::registry::FeatureRegistry;
use super::fill::{Fluids, TerrainFill};
use super::liquids::Liquids;
use super::phases::{CarverPhase, HeightsPhase, RawTerrainPhase, StartsPhase};
use super::structure_probe::{BedrockProbe, Supply};
use super::surface::SurfacePass;
use super::terrain::BedrockTerrain;
use crate::shared::blocks::{BlockId, BlockTable, Blocks};
use crate::shared::chunk::proto::{ProtoChunk, QuartBiomes};
use crate::shared::chunk::region::{Changes, Region};
use crate::shared::moss_carpet::MossCarpet;
use crate::shared::rng::seeds::level_seed32;
use crate::shared::rng::xoroshiro::Xoroshiro;
use crate::shared::shape::Shapes;
use crate::shared::structure::{Adjustments, Probed, Reaching, Remembering, Starts, StructureBlocks, StructureData, Structures, element_random};
use crate::shared::terrain::{CornerColumns, NoiseChunk, OverworldSettings, SettingsError};
use chorus_block::block_id::{LAVA, STONE, WATER};
use chorus_block::block_registry::BlockRegistry;
use chorus_level::dimension_definition::DimensionDefinition;
use chorus_level::generator::phase::{PhaseContext, Request};

const CENTRE: usize = 4;
const NO_SURFACE_LEVELS: [i32; 16] = [0; 16];
const STRUCTURE_MAGMA_TICK_DELAY: u64 = 1;

pub struct OverworldGenerator {
    pub seed: i64,
    pub(crate) terrain: BedrockTerrain,
    buffers: BufferNoises,
    fill: TerrainFill,
    pub(crate) biomes: BiomeSampler,
    pub(crate) blocks: Arc<Blocks>,
    pub(crate) water: BlockId,
    pub(crate) lava: BlockId,
    surface: SurfacePass,
    carvers: Carvers,
    pub(crate) decoration: Decoration,
    liquids: Liquids,
    structures: Option<Structures>,
    moss_carpet: MossCarpet,
}

impl OverworldGenerator {
    pub fn blocks(&self) -> &Blocks {
        &self.blocks
    }

    pub fn new(seed: i64, registry: &BlockRegistry) -> Result<Self, SettingsError> {
        Self::with_feature_registry(seed, registry, &FeatureRegistry::vanilla())
    }

    pub fn with_settings(seed: i64, registry: &BlockRegistry, settings: &OverworldSettings) -> Result<Self, SettingsError> {
        Self::with_definition(seed, registry, settings, DimensionDefinition::OVERWORLD)
    }

    pub fn with_definition(seed: i64, registry: &BlockRegistry, settings: &OverworldSettings, definition: DimensionDefinition) -> Result<Self, SettingsError> {
        let definition = definition.validate().map_err(|err| SettingsError::Invalid(err.to_string()))?;
        settings.validate()?;
        Self::assemble(seed, registry, &FeatureRegistry::vanilla(), StructureData::shared(), settings, definition)
    }

    pub fn with_feature_registry(seed: i64, registry: &BlockRegistry, features: &FeatureRegistry) -> Result<Self, SettingsError> {
        Self::assemble(seed, registry, features, StructureData::shared(), &OverworldSettings::default(), DimensionDefinition::OVERWORLD)
    }

    pub fn with_structure_registry(seed: i64, registry: &BlockRegistry, features: &FeatureRegistry, structures: Arc<StructureData>) -> Result<Self, SettingsError> {
        Self::assemble(seed, registry, features, Some(structures), &OverworldSettings::default(), DimensionDefinition::OVERWORLD)
    }

    fn assemble(
        seed: i64,
        registry: &BlockRegistry,
        features: &FeatureRegistry,
        data: Option<Arc<StructureData>>,
        settings: &OverworldSettings,
        definition: DimensionDefinition,
    ) -> Result<Self, SettingsError> {
        let data = data.filter(|_| settings.structures);
        let random = Xoroshiro::new(seed).fork_positional();
        let mut table = BlockTable::new(registry);
        let stone = table.name(STONE);
        let water = table.name(WATER);
        let lava = table.name(LAVA);
        let fill = TerrainFill::new(&random, &mut table, stone);
        let surface = SurfacePass::new(seed, &random, &mut table);
        let carvers = Carvers::new(level_seed32(seed) as u32, &mut table);
        let decoration = Decoration::new(level_seed32(seed) as u32, &mut table, &Self::feature_element_ids(data.as_deref()), features);
        let liquids = Liquids::new(&mut table);
        let moss_carpet = MossCarpet::register(&mut table);
        let structures = data.map(|data| {
            let blocks = StructureBlocks::register(&mut table, &data);
            Structures::new(seed, data, blocks)
        });
        let blocks = Arc::new(table.finish());
        if let Some(structures) = &structures {
            structures.set_blocks(blocks.clone());
        }
        let biomes = BiomeSampler::new(level_seed32(seed), settings.biome_source.clone()).map_err(|error| SettingsError::Climate(error.to_string()))?;
        Ok(Self {
            seed,
            terrain: BedrockTerrain::new(&random, settings, definition),
            buffers: BufferNoises::new(&random),
            fill,
            biomes,
            blocks,
            water,
            lava,
            surface,
            carvers,
            decoration,
            liquids,
            structures,
            moss_carpet,
        })
    }

    fn feature_element_ids(data: Option<&StructureData>) -> Vec<Arc<str>> {
        let Some(data) = data else { return Vec::new() };
        data.feature_element_names().into_iter().map(|name| Arc::from(format!("minecraft:{name}"))).collect()
    }

    fn place_structure_features(&self, region: &mut Region, owner_x: i32, owner_z: i32, base: &BedrockProbe) -> Probed<()> {
        let Some(structures) = &self.structures else { return Ok(()) };
        let probe = Remembering::new(base);
        region.settle_structure_spills();
        let (base_x, base_z) = (owner_x * 16, owner_z * 16);
        let magma_ticks = region.structure_magma_ticks().to_vec();
        for &(x, y, z) in &magma_ticks {
            let pos = IVec3::new(base_x + x as i32, y as i32, base_z + z as i32);
            region.schedule_on_chunk(pos, false, STRUCTURE_MAGMA_TICK_DELAY);
        }
        for site in structures.feature_sites(&probe, (owner_x, owner_z))? {
            let (x, y, z) = site.origin;
            self.decoration.place_named(region, &site.id, &mut element_random(site.origin), IVec3::new(x, y, z))?;
        }
        Ok(())
    }

    pub(crate) fn fill_with(&self, x: i32, z: i32, around: [&QuartBiomes; 9], noise: NoiseChunk, window: SurfaceWindow, adjustments: &Adjustments) -> (ProtoChunk, NoiseChunk, Aquifer) {
        let (min_y, height) = (self.terrain.min_y(), self.terrain.definition().height);
        let volume = self.biomes.volume(around[CENTRE].raw_samples(), x, z, min_y, height);
        let mut chunk = ProtoChunk::with_biome_volume(x, z, min_y, height, self.water, self.lava, volume);
        let aquifer = self.fill.run(&self.terrain, &noise, adjustments, &mut chunk, window, Fluids { water: self.water, lava: self.lava });
        (chunk, noise, aquifer)
    }

    pub(crate) fn surface_hints(&self, x: i32, z: i32) -> SurfaceHints {
        std::array::from_fn(|index| self.terrain.preliminary_surface(x * 4 + index as i32 / 4, z * 4 + index as i32 % 4) as i16)
    }

    pub fn quart_biomes(&self, x: i32, z: i32) -> QuartBiomes {
        QuartBiomes::new(self.biomes.samples(&self.terrain, x, z, self.terrain.min_y(), self.terrain.definition().height), NO_SURFACE_LEVELS)
    }

    pub(crate) fn corner_columns(&self, x: i32, z: i32) -> CornerColumns {
        super::columns::corner_columns(&self.terrain, &self.buffers, x, z)
    }

    pub(super) fn adjustments_at(&self, x: i32, z: i32, supply: &dyn Supply) -> Probed<Adjustments> {
        let base = BedrockProbe::reading(self, supply);
        let probe = Remembering::new(&base);
        match self.structures.as_ref() {
            Some(structures) => structures.adjustments(&probe, (x, z)),
            None => Ok(Adjustments::default()),
        }
    }

    pub(crate) fn fill_unadjusted(&self, x: i32, z: i32, around: [&QuartBiomes; 9], noise: NoiseChunk, window: SurfaceWindow) -> (ProtoChunk, NoiseChunk, Aquifer) {
        self.fill_with(x, z, around, noise, window, &Adjustments::default())
    }

    pub fn build_surface(&self, chunk: &mut ProtoChunk) {
        self.surface.build(&self.terrain, chunk);
    }

    pub fn apply_carvers(&self, aquifer: &mut Aquifer, chunk: &mut ProtoChunk) {
        self.carvers.carve(&self.blocks, &self.fill.aquifer, &self.terrain, aquifer, chunk);
        chunk.freeze_worldgen_heights(&self.blocks);
        chunk.set_fluid_ticks(aquifer.scheduled_ticks.clone());
    }

    pub fn decorate(&self, owner_x: i32, owner_z: i32, chunks: [ProtoChunk; 9], supply: &dyn Supply) -> Probed<[Changes; 9]> {
        let surface = |quart_x: i32, quart_z: i32| self.terrain.preliminary_surface(quart_x, quart_z);
        let probe = BedrockProbe::reading(self, supply);
        let structures = self.structures.as_ref();
        let in_structure = |pos: IVec3| match structures {
            Some(structures) => structures.in_structure(&probe, (pos.x, pos.y, pos.z), true),
            None => Ok(false),
        };
        let partial_template = |block: BlockId| structures.is_some_and(|structures| structures.is_partial_template(block));
        let mut region = Region::new(owner_x, owner_z, &self.blocks, chunks)
            .with_replay()
            .with_preliminary_surface(&surface)
            .with_structure_check(&in_structure)
            .with_partial_templates(&partial_template)
            .with_sea_level(self.terrain.sea_level());
        self.place_structure_features(&mut region, owner_x, owner_z, &probe)?;
        self.decoration.decorate(&mut region, owner_x, owner_z)?;
        region.begin_feature();
        let pending = self.liquids.finish_decoration(&mut region);
        region.set_pending_fluid(pending);
        Ok(region.into_changes())
    }

    pub(crate) fn shapes(&self) -> Shapes<'_> {
        Shapes {
            carpet: &self.moss_carpet,
            water: self.water,
            packed_ice_holds_snow: true,
            double_plants_keep_orphans: true,
        }
    }

    pub fn structures(&self) -> Option<&Structures> {
        self.structures.as_ref()
    }

    pub(crate) fn structure_reaching(&self, cell: (i32, i32), context: &PhaseContext<Self>) -> Probed<Option<Reaching>> {
        let base = BedrockProbe::reading(self, context);
        let probe = Remembering::new(&base);
        match self.structures.as_ref() {
            Some(structures) => structures.reaching(&probe, cell),
            None => Ok(None),
        }
    }

    fn requests_of(&self, context: &PhaseContext<Self>, raw: bool, work: impl FnOnce(&Structures, &Remembering) -> Probed<()>) -> Vec<Request<Self>> {
        let Some(structures) = &self.structures else { return Vec::new() };
        let base = if raw { BedrockProbe::for_starts(self, context) } else { BedrockProbe::reading(self, context) };
        let probe = Remembering::dry(&base);
        if let Err(missing) = work(structures, &probe) {
            warn!("a dry structure probe failed: {missing}");
        }
        let heights = probe.missed_heights().into_iter().map(|(x, z)| Request::of::<HeightsPhase>(IVec2::new(x, z)));
        let starts = probe.missed_starts().into_iter().map(|(x, z)| Request::of::<StartsPhase>(IVec2::new(x, z)));
        let chunks = probe.missed_chunks().into_iter().map(|(x, z)| {
            let cell = IVec2::new(x, z);
            if raw { Request::of::<RawTerrainPhase>(cell) } else { Request::of::<CarverPhase>(cell) }
        });
        heights.chain(starts).chain(chunks).collect()
    }

    pub(crate) fn structure_requests(&self, cell: (i32, i32), carved: &ProtoChunk, context: &PhaseContext<Self>) -> Vec<Request<Self>> {
        self.requests_of(context, false, |structures, probe| {
            if let Some(reaching) = structures.reaching(probe, cell)? {
                let mut scratch = carved.clone();
                structures.postprocess(probe, &self.blocks, &reaching, &mut scratch.edit())?;
            }
            Ok(())
        })
    }

    pub(crate) fn adjustment_requests(&self, cell: (i32, i32), context: &PhaseContext<Self>) -> Vec<Request<Self>> {
        self.requests_of(context, false, |structures, probe| structures.adjustments(probe, cell).map(drop))
    }

    pub(crate) fn decoration_requests(&self, cell: (i32, i32), context: &PhaseContext<Self>) -> Vec<Request<Self>> {
        self.requests_of(context, false, |structures, probe| {
            structures.feature_sites(probe, cell)?;
            structures.in_structure_window(probe, cell, 1)
        })
    }

    pub(crate) fn start_requests(&self, cell: (i32, i32), context: &PhaseContext<Self>) -> Vec<Request<Self>> {
        self.requests_of(context, true, |structures, probe| structures.build_starts(probe, cell).map(drop))
    }

    pub(crate) fn built_starts(&self, cell: (i32, i32), context: &PhaseContext<Self>) -> Probed<Starts> {
        let Some(structures) = &self.structures else { return Ok(Starts::default()) };
        let base = BedrockProbe::for_starts(self, context);
        structures.build_starts(&Remembering::new(&base), cell)
    }

    pub(crate) fn structure_postprocess(&self, reaching: &Reaching, chunk: &mut ProtoChunk, context: &PhaseContext<Self>) -> Probed<()> {
        let Some(structures) = &self.structures else { return Ok(()) };
        let base = BedrockProbe::reading(self, context);
        let probe = Remembering::new(&base);
        structures.postprocess(&probe, &self.blocks, reaching, chunk)
    }

    pub fn fluid_rules(&self) -> Option<chorus_level::fluid::FluidRules> {
        Some(self.liquids.rules(&self.blocks))
    }
}
