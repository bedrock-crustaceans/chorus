use crate::block::block_definition::BlockDefinition;
use crate::block::block_id;
use crate::block::state::common::{MINECRAFT_CONNECTION_EAST, MINECRAFT_CONNECTION_NORTH, MINECRAFT_CONNECTION_SOUTH, MINECRAFT_CONNECTION_WEST};
use crate::const_block;

pub const LIGHT_GRAY_STAINED_GLASS_PANE: BlockDefinition = const_block! {
    identifier: block_id::LIGHT_GRAY_STAINED_GLASS_PANE,
    states: [MINECRAFT_CONNECTION_EAST, MINECRAFT_CONNECTION_NORTH, MINECRAFT_CONNECTION_SOUTH, MINECRAFT_CONNECTION_WEST],
    components: [],
    permutations: [],
};
