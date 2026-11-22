use crate::shared::blocks::{BlockId, BlockTable, Blocks};
use chorus_block::block_id::*;

const CAVE_REPLACEABLE_FLAG: u8 = 1;
const CANYON_REPLACEABLE_FLAG: u8 = 2;
const SAND_FLAG: u8 = 4;
const DIRT_FLAG: u8 = 8;
const GRASS_OR_MYCELIUM_FLAG: u8 = 16;
const GRASS_FLAG: u8 = 32;

const SAND_BLOCKS: [&str; 3] = [SAND, RED_SAND, SUSPICIOUS_SAND];
const DIRT_BLOCKS: [&str; 2] = [DIRT, COARSE_DIRT];
const GRASS_OR_MYCELIUM_BLOCKS: [&str; 2] = [GRASS_BLOCK, MYCELIUM];
const SANDSTONES: [&str; 8] = [
    SANDSTONE,
    CHISELED_SANDSTONE,
    CUT_SANDSTONE,
    SMOOTH_SANDSTONE,
    RED_SANDSTONE,
    CHISELED_RED_SANDSTONE,
    CUT_RED_SANDSTONE,
    SMOOTH_RED_SANDSTONE,
];
const GLAZED_TERRACOTTAS: [&str; 16] = [
    WHITE_GLAZED_TERRACOTTA,
    ORANGE_GLAZED_TERRACOTTA,
    MAGENTA_GLAZED_TERRACOTTA,
    LIGHT_BLUE_GLAZED_TERRACOTTA,
    YELLOW_GLAZED_TERRACOTTA,
    LIME_GLAZED_TERRACOTTA,
    PINK_GLAZED_TERRACOTTA,
    GRAY_GLAZED_TERRACOTTA,
    SILVER_GLAZED_TERRACOTTA,
    CYAN_GLAZED_TERRACOTTA,
    PURPLE_GLAZED_TERRACOTTA,
    BLUE_GLAZED_TERRACOTTA,
    BROWN_GLAZED_TERRACOTTA,
    GREEN_GLAZED_TERRACOTTA,
    RED_GLAZED_TERRACOTTA,
    BLACK_GLAZED_TERRACOTTA,
];
const STAINED_TERRACOTTAS: [&str; 17] = [
    HARDENED_CLAY,
    WHITE_TERRACOTTA,
    ORANGE_TERRACOTTA,
    MAGENTA_TERRACOTTA,
    LIGHT_BLUE_TERRACOTTA,
    YELLOW_TERRACOTTA,
    LIME_TERRACOTTA,
    PINK_TERRACOTTA,
    GRAY_TERRACOTTA,
    LIGHT_GRAY_TERRACOTTA,
    CYAN_TERRACOTTA,
    PURPLE_TERRACOTTA,
    BLUE_TERRACOTTA,
    BROWN_TERRACOTTA,
    GREEN_TERRACOTTA,
    RED_TERRACOTTA,
    BLACK_TERRACOTTA,
];
const ORES_AND_RAW_BLOCKS: [&str; 6] = [IRON_ORE, DEEPSLATE_IRON_ORE, RAW_IRON_BLOCK, COPPER_ORE, DEEPSLATE_COPPER_ORE, RAW_COPPER_BLOCK];
const SHARED_TERRAIN: [&str; 16] = [
    STONE,
    DIRT,
    COARSE_DIRT,
    PODZOL,
    GRASS_BLOCK,
    MYCELIUM,
    SNOW_LAYER,
    PACKED_ICE,
    DEEPSLATE,
    CALCITE,
    SAND,
    RED_SAND,
    GRAVEL,
    TUFF,
    DIRT_WITH_ROOTS,
    SUSPICIOUS_SAND,
];
const CANYON_ONLY: [&str; 7] = [GRANITE, DIORITE, ANDESITE, MUD, SNOW, SULFUR, CINNABAR];

pub struct Classes {
    flags_by_kind: Vec<u8>,
    pub air: BlockId,
    pub sandstone: BlockId,
    pub grass: BlockId,
}

impl Classes {
    pub fn new(table: &mut BlockTable) -> Self {
        let mut flags_by_kind: Vec<u8> = Vec::new();
        let mut mark = |names: &[&'static str], flag: u8| {
            for &name in names {
                let kind = table.kind(name) as usize;
                if flags_by_kind.len() <= kind {
                    flags_by_kind.resize(kind + 1, 0);
                }
                flags_by_kind[kind] |= flag;
            }
        };
        mark(&SHARED_TERRAIN, CAVE_REPLACEABLE_FLAG | CANYON_REPLACEABLE_FLAG);
        mark(&SANDSTONES, CAVE_REPLACEABLE_FLAG | CANYON_REPLACEABLE_FLAG);
        mark(&ORES_AND_RAW_BLOCKS, CAVE_REPLACEABLE_FLAG | CANYON_REPLACEABLE_FLAG);
        mark(&GLAZED_TERRACOTTAS, CAVE_REPLACEABLE_FLAG);
        mark(&STAINED_TERRACOTTAS, CANYON_REPLACEABLE_FLAG);
        mark(&CANYON_ONLY, CANYON_REPLACEABLE_FLAG);
        mark(&SAND_BLOCKS, SAND_FLAG);
        mark(&DIRT_BLOCKS, DIRT_FLAG);
        mark(&GRASS_OR_MYCELIUM_BLOCKS, GRASS_OR_MYCELIUM_FLAG);
        mark(&[GRASS_BLOCK], GRASS_FLAG);
        Self {
            flags_by_kind,
            air: table.name(AIR),
            sandstone: table.name(SANDSTONE),
            grass: table.name(GRASS_BLOCK),
        }
    }

    fn has(&self, blocks: &Blocks, block: BlockId, flag: u8) -> bool {
        self.flags_by_kind.get(blocks.entry(block).kind as usize).is_some_and(|flags| flags & flag != 0)
    }

    pub fn cave_replaceable(&self, blocks: &Blocks, block: BlockId) -> bool {
        self.has(blocks, block, CAVE_REPLACEABLE_FLAG)
    }

    pub fn canyon_replaceable(&self, blocks: &Blocks, block: BlockId) -> bool {
        self.has(blocks, block, CANYON_REPLACEABLE_FLAG)
    }

    pub fn is_sand(&self, blocks: &Blocks, block: BlockId) -> bool {
        self.has(blocks, block, SAND_FLAG)
    }

    pub fn is_dirt(&self, blocks: &Blocks, block: BlockId) -> bool {
        self.has(blocks, block, DIRT_FLAG)
    }

    pub fn is_grass_or_mycelium(&self, blocks: &Blocks, block: BlockId) -> bool {
        self.has(blocks, block, GRASS_OR_MYCELIUM_FLAG)
    }

    pub fn is_grass(&self, blocks: &Blocks, block: BlockId) -> bool {
        self.has(blocks, block, GRASS_FLAG)
    }
}
