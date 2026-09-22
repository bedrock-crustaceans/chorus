use crate::block::block_definition::BlockDefinition;
use crate::block::state::common::LIT;
use crate::const_block;

pub const LIGHT_GRAY_CANDLE_CAKE: BlockDefinition = const_block! {
    identifier: "minecraft:light_gray_candle_cake",
    states: [LIT],
    components: [],
    permutations: [],
};
