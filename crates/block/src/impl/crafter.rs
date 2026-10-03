use crate::block_definition::BlockDefinition;
use crate::block_id;
use crate::component::map_color_component::MapColorComponent;
use crate::const_block;
use crate::state::common::{CRAFTING, ORIENTATION, TRIGGERED_BIT};

pub const CRAFTER: BlockDefinition = const_block! {
    identifier: block_id::CRAFTER,
    states: [CRAFTING, ORIENTATION, TRIGGERED_BIT],
    components: [
        MapColorComponent { r: 112, g: 112, b: 112, a: 255 },
    ],
    permutations: [],
};
