use std::any::Any;
use std::sync::Arc;

use super::data_driven::JigsawStructureBuilder;
use super::feature::FeatureSite;
use super::r#impl::{ancient_city, mansion, mineshaft, monument, ocean_ruin, outpost, ruined_portal, scattered, shipwreck, stronghold, treasure, village};
use super::plan::PlanWrites;
use super::{Adjustments, BoundingBox, CHUNK_CENTRE, CHUNK_WIDTH, Probed, Reaching, StartProbe, StructureKind, Structures, contains_only};
use crate::shared::blocks::BlockTable;
use crate::shared::blocks::Blocks;
use crate::shared::chunk::proto::ProtoChunk;
use chorus_structure_data::StatePoolBuilder;

pub type Built = Arc<dyn Any + Send + Sync>;

pub struct Region {
    pub bounds: BoundingBox,
    pub template: bool,
    pub settled_walls: bool,
    pub keep_template_post: bool,
}

pub struct StructureContext<'a> {
    pub structures: &'a Structures,
    pub probe: &'a dyn StartProbe,
}

pub trait StructureBuilder: Send + Sync {
    fn is_start(&self, context: &StructureContext, kind: StructureKind, chunk: (i32, i32)) -> Probed<bool>;

    fn build(&self, _context: &StructureContext, _kind: StructureKind, _chunk: (i32, i32)) -> Probed<Option<Built>> {
        Ok(None)
    }

    fn reaches(&self, _context: &StructureContext, _kind: StructureKind, _start: (i32, i32), _area: &BoundingBox) -> Probed<bool> {
        Ok(true)
    }

    fn regions(&self, _context: &StructureContext, _blocks: &Blocks, _kind: StructureKind, _start: (i32, i32)) -> Probed<Vec<Region>> {
        Ok(Vec::new())
    }

    fn adjust(&self, _context: &StructureContext, _chunk: (i32, i32), _adjustments: &mut Adjustments) -> Probed<()> {
        Ok(())
    }

    fn contains(&self, _context: &StructureContext, _chunk: (i32, i32), _at: (i32, i32, i32), _with_boxes: bool) -> Probed<bool> {
        Ok(false)
    }

    fn near_window(&self, _context: &StructureContext, _chunk: (i32, i32), _margin: i32) -> Probed<()> {
        Ok(())
    }

    fn feature_sites(&self, _context: &StructureContext, _chunk: (i32, i32), _area: &BoundingBox, _sites: &mut Vec<FeatureSite>) -> Probed<()> {
        Ok(())
    }

    fn starts_near(&self, _context: &StructureContext, _kind: StructureKind, _centre: (i32, i32), _radius: i32) -> Probed<Option<Vec<(i32, i32)>>> {
        Ok(None)
    }

    fn plan(&self, _context: &StructureContext, _blocks: &Blocks, _kind: StructureKind, _start: (i32, i32)) -> Probed<Option<Arc<PlanWrites>>> {
        Ok(None)
    }

    fn place(&self, context: &StructureContext, blocks: &Blocks, reaching: &Reaching, chunk: &mut ProtoChunk) -> Probed<()>;

    fn neighbour_rank(&self) -> Option<u8> {
        None
    }
}

pub(super) fn erase<T: Any + Send + Sync>(value: T) -> Built {
    Arc::new(value)
}

pub(super) fn surface_centre(context: &StructureContext, chunk: (i32, i32)) -> (i32, i32, i32) {
    (chunk.0 * CHUNK_WIDTH + CHUNK_CENTRE, context.probe.preliminary_surface(chunk), chunk.1 * CHUNK_WIDTH + CHUNK_CENTRE)
}

pub(super) fn passes_filter(context: &StructureContext, kind: StructureKind, chunk: (i32, i32)) -> bool {
    let registry = context.structures.registry();
    let Some(filter) = registry.filter(kind) else { return true };
    contains_only(context.probe, surface_centre(context, chunk), filter.radius, registry.biomes(&filter.tag))
}

pub(super) fn covers<'a>(mut pieces: impl Iterator<Item = &'a BoundingBox>, area: &BoundingBox) -> bool {
    pieces.any(|bounds| bounds.intersects(area))
}

pub(super) fn place_plan(context: &StructureContext, blocks: &Blocks, kind: StructureKind, start: (i32, i32), chunk: &mut ProtoChunk) -> Probed<()> {
    let Some(plan) = context.structures.plan(context.probe, blocks, kind, start)? else {
        return Ok(());
    };
    context.structures.apply(blocks, chunk, kind, &plan);
    Ok(())
}

pub type DeclareBlocks = Arc<dyn Fn(&mut BlockTable) + Send + Sync>;
pub type DeclareStates = Arc<dyn Fn(&mut StatePoolBuilder) + Send + Sync>;

#[derive(Clone)]
pub struct StructureRegistration {
    pub identifier: String,
    pub builder: Arc<dyn StructureBuilder>,
    pub declare_blocks: DeclareBlocks,
    pub declare_states: DeclareStates,
}

impl StructureRegistration {
    pub fn new(identifier: impl Into<String>, builder: Arc<dyn StructureBuilder>) -> Self {
        Self {
            identifier: identifier.into(),
            builder,
            declare_blocks: Arc::new(|_| {}),
            declare_states: Arc::new(|_| {}),
        }
    }

    pub fn declaring_blocks(mut self, declare: impl Fn(&mut BlockTable) + Send + Sync + 'static) -> Self {
        self.declare_blocks = Arc::new(declare);
        self
    }
}

impl StructureRegistration {
    pub fn declaring_states(mut self, declare: impl Fn(&mut StatePoolBuilder) + Send + Sync + 'static) -> Self {
        self.declare_states = Arc::new(declare);
        self
    }
}

pub fn built_in_registrations() -> Vec<StructureRegistration> {
    let entry = |identifier: &str, builder: Arc<dyn StructureBuilder>| StructureRegistration::new(identifier, builder);
    vec![
        entry("village", Arc::new(village::VillageBuilder)),
        entry("stronghold", Arc::new(stronghold::StrongholdBuilder)),
        entry("buried_treasure", Arc::new(treasure::BuriedTreasureBuilder)),
        entry("ruined_portal", Arc::new(ruined_portal::RuinedPortalBuilder)).declaring_states(ruined_portal::declare_states),
        entry("temple", Arc::new(scattered::TempleBuilder)),
        entry("mineshaft", Arc::new(mineshaft::MineshaftBuilder)),
        entry("shipwreck", Arc::new(shipwreck::ShipwreckBuilder)).declaring_states(shipwreck::declare_wood_states),
        entry("ocean_ruins", Arc::new(ocean_ruin::OceanRuinsBuilder)),
        entry("trial_chambers", Arc::new(JigsawStructureBuilder::trial_chambers())),
        entry("trail_ruins", Arc::new(JigsawStructureBuilder::trail_ruins())),
        entry("abandoned_camp", Arc::new(JigsawStructureBuilder::abandoned_camp())),
        entry("ancient_city", Arc::new(ancient_city::AncientCityBuilder)),
        entry("outpost", Arc::new(outpost::OutpostBuilder)),
        entry("monument", Arc::new(monument::MonumentBuilder)),
        entry("mansion", Arc::new(mansion::MansionBuilder)),
    ]
}
