use super::connections::Connections;
use super::data::StructureData;
use super::r#impl::mansion::MansionBlocks;
use super::r#impl::mineshaft::MineshaftBlocks;
use super::r#impl::monument::MonumentBlocks;
use super::r#impl::scattered::{Material, ScatteredBlocks};
use super::r#impl::stronghold::StrongholdBlocks;
use super::r#impl::village::VillageBlocks;
use super::palette::PaletteBlocks;
use crate::shared::blocks::{AIR, BlockId, BlockTable, state, text};
use chorus_block::block_id::{CHEST, COBBLESTONE, DEEPSLATE, DIRT, LAVA, STONE, WATER};
use chorus_block::state::common::MINECRAFT_CARDINAL_DIRECTION;
use std::sync::Arc;

#[derive(Clone, Default)]
pub struct StructureBlocks {
    pub chest_north: BlockId,
    pub air: BlockId,
    pub stone: BlockId,
    pub deepslate: BlockId,
    pub dirt: BlockId,
    pub cobblestone: BlockId,
    pub water: BlockId,
    pub lava: BlockId,
    pub palette: Option<Arc<PaletteBlocks>>,
    pub scattered: Arc<ScatteredBlocks>,
    pub mineshaft: Arc<MineshaftBlocks>,
    pub mansion: Arc<MansionBlocks>,
    pub monument: Arc<MonumentBlocks>,
    pub stronghold: Arc<StrongholdBlocks>,
    pub village: Arc<VillageBlocks>,
    pub connections: Arc<Connections>,
}

impl StructureBlocks {
    pub fn scattered_id(&self, material: Material) -> BlockId {
        self.scattered.id(material)
    }

    pub fn register(table: &mut BlockTable, data: &StructureData) -> Self {
        let mut blocks = Self {
            chest_north: table.get(CHEST, &[state(&MINECRAFT_CARDINAL_DIRECTION, text("north"))]),
            air: AIR,
            stone: table.name(STONE),
            deepslate: table.name(DEEPSLATE),
            dirt: table.name(DIRT),
            cobblestone: table.name(COBBLESTONE),
            water: table.name(WATER),
            lava: table.name(LAVA),
            palette: data.library.as_ref().map(|library| Arc::new(PaletteBlocks::register(table, library))),
            scattered: Arc::new(ScatteredBlocks::register(table)),
            mineshaft: Arc::new(MineshaftBlocks::register(table)),
            mansion: Arc::new(MansionBlocks::register(table)),
            monument: Arc::new(MonumentBlocks::register(table)),
            stronghold: Arc::new(StrongholdBlocks::register(table)),
            village: Arc::new(VillageBlocks::register(table)),
            connections: Arc::default(),
        };
        blocks.connections = Arc::new(Connections::register(table));
        for (_, registration) in data.registry.registrations() {
            (registration.declare_blocks)(table);
        }
        blocks
    }
}

pub(crate) const CHESTS: [&str; 2] = ["chest", "trapped_chest"];
const FALLING: [&str; 6] = ["sand", "red_sand", "gravel", "suspicious_sand", "suspicious_gravel", "dragon_egg"];
const FALLING_SUFFIXES: [&str; 2] = ["concrete_powder", "anvil"];

pub(crate) fn is_falling_block(name: &str) -> bool {
    let name = name.strip_prefix("minecraft:").unwrap_or(name);
    FALLING.contains(&name) || FALLING_SUFFIXES.iter().any(|suffix| name.ends_with(suffix))
}
