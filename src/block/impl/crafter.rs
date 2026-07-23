use crate::block::block_definition::BlockDefinition;
use crate::block::component::map_color_component::MapColorComponent;
use crate::block::state::common::{CRAFTING, ORIENTATION, TRIGGERED_BIT};
use crate::const_block;

pub const CRAFTER: BlockDefinition = const_block! {
    identifier: "minecraft:crafter",
    states: [CRAFTING, ORIENTATION, TRIGGERED_BIT],
    components: [
        MapColorComponent { r: 112, g: 112, b: 112, a: 255 },
    ],
    permutations: [],
};
