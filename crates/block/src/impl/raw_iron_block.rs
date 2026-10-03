use crate::block_definition::BlockDefinition;
use crate::block_id;
use crate::component::map_color_component::MapColorComponent;
use crate::const_block;

pub const RAW_IRON_BLOCK: BlockDefinition = const_block! {
    identifier: block_id::RAW_IRON_BLOCK,
    states: [],
    components: [
        MapColorComponent { r: 216, g: 175, b: 147, a: 255 },
    ],
    permutations: [],
};
