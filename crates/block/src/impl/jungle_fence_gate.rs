use crate::block_definition::BlockDefinition;
use crate::block_id;
use crate::const_block;
use crate::state::common::{IN_WALL_BIT, MINECRAFT_CARDINAL_DIRECTION, OPEN_BIT};

pub const JUNGLE_FENCE_GATE: BlockDefinition = const_block! {
    identifier: block_id::JUNGLE_FENCE_GATE,
    states: [IN_WALL_BIT, MINECRAFT_CARDINAL_DIRECTION, OPEN_BIT],
    components: [],
    permutations: [],
};
