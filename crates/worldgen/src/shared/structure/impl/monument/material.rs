use crate::shared::blocks::{AIR, BlockId, BlockTable};
use chorus_block::block_id::{DARK_PRISMARINE, GOLD_BLOCK, PRISMARINE, PRISMARINE_BRICKS, SEA_LANTERN, WATER, WET_SPONGE};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(usize)]
pub enum Material {
    Air,
    Water,
    Prismarine,
    Bricks,
    Dark,
    Lantern,
    WetSponge,
    Gold,
}

const NAMES: [&str; 7] = [WATER, PRISMARINE, PRISMARINE_BRICKS, DARK_PRISMARINE, SEA_LANTERN, WET_SPONGE, GOLD_BLOCK];

#[derive(Clone, Default)]
pub struct MonumentBlocks {
    ids: [BlockId; 8],
}

impl MonumentBlocks {
    pub fn register(table: &mut BlockTable) -> Self {
        let mut ids = [AIR; 8];
        for (slot, name) in ids[1..].iter_mut().zip(NAMES) {
            *slot = table.name(name);
        }
        Self { ids }
    }

    pub fn id(&self, material: Material) -> BlockId {
        self.ids[material as usize]
    }
}
