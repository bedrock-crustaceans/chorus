mod flow;
mod ticks;

pub use ticks::{FluidTicks, ScheduledUpdate};

pub const DEPTHS: usize = 16;
pub const WATER_DELAY: u64 = 5;
pub const WATER_DECAY: i32 = 1;
pub const LAVA_DELAY: u64 = 30;
pub const LAVA_DECAY: i32 = 2;
pub const NETHER_LAVA_DELAY: u64 = 10;
pub const NETHER_LAVA_DECAY: i32 = 1;
pub const SETTLE_TICKS: usize = 20;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LiquidMotion {
    pub delay: u64,
    pub decay: i32,
}

impl LiquidMotion {
    pub const WATER: Self = Self {
        delay: WATER_DELAY,
        decay: WATER_DECAY,
    };
    pub const LAVA: Self = Self { delay: LAVA_DELAY, decay: LAVA_DECAY };
    pub const FAST_LAVA: Self = Self {
        delay: NETHER_LAVA_DELAY,
        decay: NETHER_LAVA_DECAY,
    };
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Liquid {
    pub lava: bool,
    pub depth: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PendingUpdate {
    pub pos: glam::IVec3,
    pub lava: bool,
    pub delay: u64,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct CellRules {
    pub liquid: Option<Liquid>,
    pub waterlogged: bool,
    pub stops_water: bool,
    pub stops_lava: bool,
    pub solid: bool,
    pub water_source: bool,
    pub magma: bool,
}

#[derive(Clone, Copy, Debug)]
pub struct FluidBlocks {
    pub air: i32,
    pub stone: i32,
    pub obsidian: i32,
    pub cobblestone: i32,
    pub bubble_column: i32,
    pub water: [i32; DEPTHS],
    pub lava: [i32; DEPTHS],
    pub water_motion: LiquidMotion,
    pub lava_motion: LiquidMotion,
}

pub struct FluidRules {
    blocks: FluidBlocks,
    cells: rustc_hash::FxHashMap<i32, CellRules>,
}

impl FluidRules {
    pub fn new(blocks: FluidBlocks, cells: rustc_hash::FxHashMap<i32, CellRules>) -> Self {
        Self { blocks, cells }
    }

    pub fn blocks(&self) -> &FluidBlocks {
        &self.blocks
    }

    pub fn cell(&self, block: i32) -> CellRules {
        self.cells.get(&block).copied().unwrap_or_default()
    }

    pub fn liquid(&self, block: i32) -> Option<Liquid> {
        self.cell(block).liquid
    }

    pub fn motion(&self, lava: bool) -> LiquidMotion {
        if lava { self.blocks.lava_motion } else { self.blocks.water_motion }
    }

    pub fn liquid_block(&self, lava: bool, depth: u8) -> i32 {
        let table = if lava { &self.blocks.lava } else { &self.blocks.water };
        table[(depth as usize).min(DEPTHS - 1)]
    }
}

pub trait Grid {
    fn read(&self, pos: glam::IVec3) -> Option<i32>;

    fn write(&mut self, pos: glam::IVec3, block: i32) -> bool;

    fn can_tick(&self, pos: glam::IVec3) -> bool;

    fn route(&mut self, _pos: glam::IVec3, _lava: bool, _delay: u64) -> bool {
        false
    }
}
