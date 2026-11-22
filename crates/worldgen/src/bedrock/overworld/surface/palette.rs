use crate::shared::blocks::{AIR, BlockId, BlockTable};
use chorus_block::block_id::*;

const FOUNDATION_BLOCKS: [&str; 7] = [STONE, GRANITE, POLISHED_GRANITE, DIORITE, POLISHED_DIORITE, ANDESITE, POLISHED_ANDESITE];
pub const TERRACOTTA_COLORS: usize = 16;
const TERRACOTTA: [&str; TERRACOTTA_COLORS] = [
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
pub const WHITE: usize = 0;
pub const ORANGE: usize = 1;
pub const YELLOW: usize = 4;
pub const BROWN: usize = 12;
pub const RED: usize = 14;

pub struct Palette {
    pub air: BlockId,
    pub bedrock: BlockId,
    pub stone: BlockId,
    pub water: BlockId,
    pub lava: BlockId,
    pub sand: BlockId,
    pub red_sand: BlockId,
    pub suspicious_sand: BlockId,
    pub sandstone: BlockId,
    pub red_sandstone: BlockId,
    pub gravel: BlockId,
    pub ice: BlockId,
    pub packed_ice: BlockId,
    pub blue_ice: BlockId,
    pub snow_block: BlockId,
    pub coarse_dirt: BlockId,
    pub grass_block: BlockId,
    pub netherrack: BlockId,
    pub terracotta: [BlockId; TERRACOTTA_COLORS],
    foundation: [BlockId; FOUNDATION_BLOCKS.len()],
}

impl Palette {
    pub fn new(table: &mut BlockTable) -> Self {
        let mut block = |name: &'static str| table.name(name);
        Self {
            air: AIR,
            bedrock: block(BEDROCK),
            stone: block(STONE),
            water: block(WATER),
            lava: block(LAVA),
            sand: block(SAND),
            red_sand: block(RED_SAND),
            suspicious_sand: block(SUSPICIOUS_SAND),
            sandstone: block(SANDSTONE),
            red_sandstone: block(RED_SANDSTONE),
            gravel: block(GRAVEL),
            ice: block(ICE),
            packed_ice: block(PACKED_ICE),
            blue_ice: block(BLUE_ICE),
            snow_block: block(SNOW),
            coarse_dirt: block(COARSE_DIRT),
            grass_block: block(GRASS_BLOCK),
            netherrack: block(NETHERRACK),
            terracotta: TERRACOTTA.map(&mut block),
            foundation: FOUNDATION_BLOCKS.map(&mut block),
        }
    }

    pub fn is_foundation(&self, block: BlockId) -> bool {
        self.foundation.contains(&block)
    }

    pub fn is_liquid(&self, block: BlockId) -> bool {
        block == self.water || block == self.lava
    }

    pub fn is_sand(&self, block: BlockId) -> bool {
        block == self.sand || block == self.red_sand || block == self.suspicious_sand
    }
}
