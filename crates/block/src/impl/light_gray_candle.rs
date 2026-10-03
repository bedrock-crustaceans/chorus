use crate::block_definition::BlockDefinition;
use crate::block_id;
use crate::const_block;
use crate::state::common::{CANDLES, LIT};

pub const LIGHT_GRAY_CANDLE: BlockDefinition = const_block! {
    identifier: block_id::LIGHT_GRAY_CANDLE,
    states: [CANDLES, LIT],
    components: [],
    permutations: [],
};
