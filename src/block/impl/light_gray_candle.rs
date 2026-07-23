use crate::block::block_definition::BlockDefinition;
use crate::block::state::common::{CANDLES, LIT};
use crate::const_block;

pub const LIGHT_GRAY_CANDLE: BlockDefinition = const_block! {
    identifier: "minecraft:light_gray_candle",
    states: [CANDLES, LIT],
    components: [],
    permutations: [],
};
