use crate::block_definition::BlockDefinition;
use crate::block_id;
use crate::const_block;
use crate::state::common::{MINECRAFT_CONNECTION_EAST, MINECRAFT_CONNECTION_NORTH, MINECRAFT_CONNECTION_SOUTH, MINECRAFT_CONNECTION_WEST};

pub const HARD_LIGHT_GRAY_STAINED_GLASS_PANE: BlockDefinition = const_block! {
    identifier: block_id::HARD_LIGHT_GRAY_STAINED_GLASS_PANE,
    states: [MINECRAFT_CONNECTION_EAST, MINECRAFT_CONNECTION_NORTH, MINECRAFT_CONNECTION_SOUTH, MINECRAFT_CONNECTION_WEST],
    components: [],
    permutations: [],
};
