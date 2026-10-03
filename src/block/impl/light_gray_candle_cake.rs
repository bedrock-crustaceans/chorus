use crate::block::block_definition::BlockDefinition;
use crate::block::block_id;
use crate::block::state::common::LIT;
use crate::const_block;

pub const LIGHT_GRAY_CANDLE_CAKE: BlockDefinition = const_block! {
    identifier: block_id::LIGHT_GRAY_CANDLE_CAKE,
    states: [LIT],
    components: [],
    permutations: [],
};
