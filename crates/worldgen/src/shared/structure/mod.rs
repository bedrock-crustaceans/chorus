mod adjust;
mod blocks;
mod bounds;
mod builder;
mod connections;
pub mod data;
mod data_driven;
mod entities;
mod feature;
pub mod r#impl;
mod kind;
pub mod layout;
mod locate;
mod palette;
mod place;
mod placement;
mod plan;
mod probe;
mod reaching;
mod registry;
mod seed;
mod starts;
mod template;

use std::sync::{Arc, OnceLock};

pub use adjust::Adjustments;
pub use blocks::StructureBlocks;
pub(crate) use blocks::{CHESTS, is_falling_block};
pub use bounds::BoundingBox;
pub use builder::{DeclareBlocks, StructureBuilder, StructureContext, StructureRegistration, built_in_registrations};
pub use data::StructureData;
pub use data::pack_biome_id;
pub use data_driven::{JigsawStructureBuilder, StartRule};
pub use entities::{PlacedEntity, entities_at, report_structure_entities};
pub use feature::element_random;
pub use kind::StructureKind;
pub use palette::{PaletteBlocks, register_named};
pub use place::{Library, Rotation};
pub use placement::MineshaftSeed;
pub use placement::{Grid, Spread};
pub use plan::{ColumnHeights, column_heights_of};
pub use probe::{MissingInput, NO_SURFACE, Probed, Remembering, StartProbe, contains_only};
pub use reaching::Reaching;
pub use registry::{PlacementRule, StructureDef, StructureRegistry};
pub(crate) use seed::LargeFeatureSeed;
pub use starts::Starts;
pub use template::{StateKind, Template, TemplateLibrary};

use crate::shared::CHUNK_WIDTH;
use crate::shared::blocks::{BlockId, Blocks};
use template::FULL_CUBE;

const CHUNK_CENTRE: i32 = 8;
const MONUMENT_DEEP_RADIUS: i32 = 16;
const MONUMENT_SURROUNDING_RADIUS: i32 = 29;
const SHIPWRECK_RADIUS: i32 = 20;
const SHIPWRECK_BEACH_RADIUS: i32 = 10;
const ANCIENT_CITY_Y: i32 = -27;
const MONUMENT_EXCLUSION_BEFORE: i32 = 5;
const MONUMENT_EXCLUSION_AFTER: i32 = 4;

pub struct Structures {
    seed: i64,
    level_seed: u32,
    mineshaft: MineshaftSeed,
    stronghold_spiral: OnceLock<Vec<(i32, i32)>>,
    data: Arc<StructureData>,
    registry: StructureRegistry,
    blocks: StructureBlocks,
    seeds: seed::LargeFeatureSeed,
    builders: Vec<Arc<dyn StructureBuilder>>,
    kinds: Vec<StructureKind>,
    adjust_order: Vec<usize>,
    table: OnceLock<Arc<Blocks>>,
}

impl Structures {
    pub fn load(seed: i64, blocks: StructureBlocks) -> Option<Self> {
        StructureData::shared().map(|data| Self::new(seed, data, blocks))
    }

    pub fn new(seed: i64, data: Arc<StructureData>, blocks: StructureBlocks) -> Self {
        Self::with_limit(seed, data, blocks, 0)
    }

    pub fn with_limit(seed: i64, data: Arc<StructureData>, blocks: StructureBlocks, _limit: usize) -> Self {
        let level_seed = seed as u32;
        let (kinds, builders): (Vec<StructureKind>, Vec<Arc<dyn StructureBuilder>>) = data.registry.registrations().map(|(kind, registration)| (kind, registration.builder.clone())).unzip();
        let mut structures = Self {
            seed,
            level_seed,
            mineshaft: MineshaftSeed::new(level_seed),
            stronghold_spiral: OnceLock::new(),
            registry: data.registry.clone(),
            data,
            blocks,
            seeds: seed::LargeFeatureSeed::new(level_seed),
            builders,
            kinds,
            adjust_order: Vec::new(),
            table: OnceLock::new(),
        };
        structures.adjust_order = structures.adjust_order();
        structures
    }

    pub fn set_blocks(&self, blocks: Arc<Blocks>) {
        let _ = self.table.set(blocks);
    }

    pub fn builders(&self) -> &[Arc<dyn StructureBuilder>] {
        &self.builders
    }

    pub fn kinds(&self) -> Vec<StructureKind> {
        self.kinds.clone()
    }

    pub(super) fn builder_index(&self, kind: StructureKind) -> Option<usize> {
        self.kinds.iter().position(|known| *known == kind)
    }

    pub fn data(&self) -> &Arc<StructureData> {
        &self.data
    }

    pub fn blocks(&self) -> &StructureBlocks {
        &self.blocks
    }

    pub fn seeds(&self) -> &seed::LargeFeatureSeed {
        &self.seeds
    }

    pub fn level_seed(&self) -> u32 {
        self.level_seed
    }

    pub fn is_partial_template(&self, block: BlockId) -> bool {
        self.blocks.palette.as_ref().is_some_and(|palette| palette.flags_of(block).is_some_and(|flags| flags & FULL_CUBE == 0))
    }

    pub fn registry(&self) -> &StructureRegistry {
        &self.registry
    }

    pub fn tag(&self, name: &str) -> &[i32] {
        self.registry.biomes(name)
    }

    pub fn candidate(&self, kind: StructureKind, chunk: (i32, i32)) -> Option<(i32, i32)> {
        match (self.registry().grid(kind), self.registry().java_grid(kind)) {
            (Some(grid), _) => Some(grid.candidate(self.level_seed, chunk)),
            (None, Some(grid)) => Some(grid.java_candidate(self.seed, chunk)),
            (None, None) => None,
        }
    }

    pub fn is_start(&self, probe: &dyn StartProbe, kind: StructureKind, chunk: (i32, i32)) -> Probed<bool> {
        if let Some(candidate) = self.candidate(kind, chunk)
            && candidate != chunk
        {
            return Ok(false);
        }
        let Some(index) = self.builder_index(kind) else { return Ok(false) };
        self.builders[index].is_start(&StructureContext { structures: self, probe }, kind, chunk)
    }

    pub fn stronghold_spiral(&self, probe: &dyn StartProbe) -> Probed<&[(i32, i32)]> {
        if let Some(known) = self.stronghold_spiral.get() {
            return Ok(known);
        }
        let spiral = r#impl::stronghold::spiral_starts(self.level_seed, &|chunk| self.is_start(probe, StructureKind::Village, chunk))?;
        Ok(self.stronghold_spiral.get_or_init(|| spiral))
    }

    fn monument_biomes(&self, probe: &dyn StartProbe, chunk: (i32, i32)) -> bool {
        let centre = (chunk.0 * CHUNK_WIDTH + CHUNK_CENTRE, probe.preliminary_surface(chunk), chunk.1 * CHUNK_WIDTH + CHUNK_CENTRE);
        contains_only(probe, centre, MONUMENT_DEEP_RADIUS, self.tag("deep_oceans")) && contains_only(probe, centre, MONUMENT_SURROUNDING_RADIUS, self.tag("monument_surroundings"))
    }

    fn near_monument(&self, probe: &dyn StartProbe, (x, z): (i32, i32)) -> Probed<bool> {
        let (x0, x1) = (x - MONUMENT_EXCLUSION_BEFORE, x + MONUMENT_EXCLUSION_AFTER);
        let (z0, z1) = (z - MONUMENT_EXCLUSION_BEFORE, z + MONUMENT_EXCLUSION_AFTER);
        let grid = self.registry().grid(StructureKind::Monument).expect("the monument has a placement grid");
        let spacing = grid.spacing;
        for region_x in x0.div_euclid(spacing)..=x1.div_euclid(spacing) {
            for region_z in z0.div_euclid(spacing)..=z1.div_euclid(spacing) {
                let candidate = grid.candidate(self.level_seed, (region_x * spacing, region_z * spacing));
                if (x0..=x1).contains(&candidate.0) && (z0..=z1).contains(&candidate.1) && self.grid_start(probe, StructureKind::Monument, candidate)? {
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }

    fn shipwreck_biomes(&self, probe: &dyn StartProbe, chunk: (i32, i32)) -> bool {
        let centre = (chunk.0 * CHUNK_WIDTH + CHUNK_CENTRE, probe.preliminary_surface(chunk), chunk.1 * CHUNK_WIDTH + CHUNK_CENTRE);
        let biome = probe.biome(centre.0, centre.1, centre.2);
        let radius = if self.tag("beaches").contains(&biome) { SHIPWRECK_BEACH_RADIUS } else { SHIPWRECK_RADIUS };
        contains_only(probe, centre, 0, self.tag("shipwreck")) && contains_only(probe, centre, radius, &[biome])
    }

    pub fn grid_start(&self, probe: &dyn StartProbe, kind: StructureKind, chunk: (i32, i32)) -> Probed<bool> {
        Ok(self.starts_at(probe, chunk)?.has(kind))
    }

    pub fn starts_near(&self, probe: &dyn StartProbe, kind: StructureKind, centre: (i32, i32), radius: i32) -> Probed<Vec<(i32, i32)>> {
        let (x0, z0, x1, z1) = (centre.0 - radius, centre.1 - radius, centre.0 + radius, centre.1 + radius);
        let inside = |(x, z): (i32, i32)| (x0..=x1).contains(&x) && (z0..=z1).contains(&z);
        if let Some(grid) = self.registry().any_grid(kind) {
            let spacing = grid.spacing;
            let regions = |low: i32, high: i32| low.div_euclid(spacing)..=high.div_euclid(spacing);
            let mut found = Vec::new();
            for region_x in regions(x0, x1) {
                for region_z in regions(z0, z1) {
                    let Some(candidate) = self.candidate(kind, (region_x * spacing, region_z * spacing)) else {
                        continue;
                    };
                    if inside(candidate) && self.grid_start(probe, kind, candidate)? {
                        found.push(candidate);
                    }
                }
            }
            return Ok(found);
        }
        if let Some(index) = self.builder_index(kind)
            && let Some(found) = self.builders[index].starts_near(&StructureContext { structures: self, probe }, kind, centre, radius)?
        {
            self.require_starts(probe, &found)?;
            return Ok(found);
        }
        let mut found: Vec<(i32, i32)> = Vec::new();
        for x in x0..=x1 {
            for z in z0..=z1 {
                if self.is_start(probe, kind, (x, z))? {
                    found.push((x, z));
                }
            }
        }
        self.require_starts(probe, &found)?;
        Ok(found)
    }

    fn require_starts(&self, probe: &dyn StartProbe, cells: &[(i32, i32)]) -> Probed<()> {
        for &cell in cells {
            self.starts_at(probe, cell)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod pinned_dump;
#[cfg(test)]
mod tests;
