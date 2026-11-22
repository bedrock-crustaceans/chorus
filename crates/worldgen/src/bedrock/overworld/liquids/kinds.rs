use crate::shared::blocks::{BlockId, Blocks};
use chorus_block::block_id::{FLOWING_LAVA, FLOWING_WATER, LAVA, WATER};
use chorus_block::component::liquid_detection_component::LiquidTouch;
use chorus_block::state::common::LIQUID_DEPTH;
use chorus_level::fluid::Liquid;

pub struct Kinds {
    liquid: Vec<Option<Liquid>>,
    named_liquid: Vec<bool>,
    waterlogged: Vec<bool>,
    stops_water: Vec<bool>,
    stops_lava: Vec<bool>,
    solid: Vec<bool>,
}

impl Kinds {
    pub fn new(blocks: &Blocks) -> Self {
        let entries = blocks.entries();
        let mut kinds = Self {
            liquid: Vec::with_capacity(entries.len()),
            named_liquid: Vec::with_capacity(entries.len()),
            waterlogged: Vec::with_capacity(entries.len()),
            stops_water: Vec::with_capacity(entries.len()),
            stops_lava: Vec::with_capacity(entries.len()),
            solid: Vec::with_capacity(entries.len()),
        };
        for (id, entry) in entries.iter().enumerate() {
            let id = id as BlockId;
            let lava = entry.name.as_ref() == LAVA || entry.name.as_ref() == FLOWING_LAVA;
            let named = lava || entry.name.as_ref() == WATER || entry.name.as_ref() == FLOWING_WATER;
            let depth = blocks.int(id, &LIQUID_DEPTH).unwrap_or(0).clamp(0, 15) as u8;
            let waterlogged = entry.flooded && !named;
            let liquid = if named {
                Some(Liquid { lava, depth })
            } else if waterlogged {
                Some(Liquid { lava: false, depth: 0 })
            } else {
                None
            };
            kinds.liquid.push(liquid);
            kinds.named_liquid.push(named);
            kinds.waterlogged.push(waterlogged);
            kinds.stops_water.push(!named && entry.water_touch == LiquidTouch::Blocking);
            kinds.stops_lava.push(!named && entry.lava_touch == LiquidTouch::Blocking);
            kinds.solid.push(entry.solid);
        }
        kinds
    }

    pub fn liquid(&self, block: BlockId) -> Option<Liquid> {
        self.liquid.get(block as usize).copied().flatten()
    }

    pub fn is_waterlogged(&self, block: BlockId) -> bool {
        self.waterlogged.get(block as usize).copied().unwrap_or(false)
    }

    pub fn is_named_water(&self, block: BlockId) -> bool {
        self.named_liquid.get(block as usize).copied().unwrap_or(false) && self.liquid(block).is_some_and(|liquid| !liquid.lava)
    }

    pub fn is_water_source_block(&self, block: BlockId) -> bool {
        self.named_liquid.get(block as usize).copied().unwrap_or(false) && self.liquid(block) == Some(Liquid { lava: false, depth: 0 })
    }

    pub fn stops(&self, block: BlockId, lava: bool) -> bool {
        let table = if lava { &self.stops_lava } else { &self.stops_water };
        table.get(block as usize).copied().unwrap_or(false)
    }

    pub fn is_solid(&self, block: BlockId) -> bool {
        self.solid.get(block as usize).copied().unwrap_or(false)
    }
}
