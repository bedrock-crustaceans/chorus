use crate::block_definition::BlockDefinition;
use crate::block_id;
use crate::const_block;
use crate::state::common::LIT;

pub const LIGHT_GRAY_CANDLE_CAKE: BlockDefinition = const_block! {
    identifier: block_id::LIGHT_GRAY_CANDLE_CAKE,
    states: [LIT],
    components: [],
    permutations: [],
};
