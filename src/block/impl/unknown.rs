use crate::block::block_definition::BlockDefinition;
use crate::block::block_id;
use crate::const_block;

pub const UNKNOWN: BlockDefinition = const_block! {
    identifier: block_id::UNKNOWN,
    states: [],
    components: [],
    permutations: [],
};
