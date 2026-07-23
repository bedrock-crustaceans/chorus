use crate::block::block_definition::BlockDefinition;
use crate::block::component::map_color_component::MapColorComponent;
use crate::block::state::common::{MINECRAFT_CARDINAL_DIRECTION, OMINOUS, VAULT_STATE};
use crate::const_block;

pub const VAULT: BlockDefinition = const_block! {
    identifier: "minecraft:vault",
    states: [MINECRAFT_CARDINAL_DIRECTION, OMINOUS, VAULT_STATE],
    components: [
        MapColorComponent { r: 112, g: 112, b: 112, a: 255 },
    ],
    permutations: [],
};
