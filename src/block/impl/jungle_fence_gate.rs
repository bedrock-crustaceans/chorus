use crate::block::block_definition::BlockDefinition;
use crate::block::state::common::{IN_WALL_BIT, MINECRAFT_CARDINAL_DIRECTION, OPEN_BIT};
use crate::const_block;

pub const JUNGLE_FENCE_GATE: BlockDefinition = const_block! {
    identifier: "minecraft:jungle_fence_gate",
    states: [IN_WALL_BIT, MINECRAFT_CARDINAL_DIRECTION, OPEN_BIT],
    components: [],
    permutations: [],
};
