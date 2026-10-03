use crate::block_definition::BlockDefinition;
use crate::block_id;
use crate::component::map_color_component::MapColorComponent;
use crate::const_block;
use crate::state::common::{MINECRAFT_CARDINAL_DIRECTION, OMINOUS, VAULT_STATE};

pub const VAULT: BlockDefinition = const_block! {
    identifier: block_id::VAULT,
    states: [MINECRAFT_CARDINAL_DIRECTION, OMINOUS, VAULT_STATE],
    components: [
        MapColorComponent { r: 112, g: 112, b: 112, a: 255 },
    ],
    permutations: [],
};
