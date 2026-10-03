use crate::block::block_definition::BlockDefinition;
use crate::block::block_id;
use crate::block::component::map_color_component::MapColorComponent;
use crate::const_block;

pub const INFO_UPDATE: BlockDefinition = const_block! {
    identifier: block_id::INFO_UPDATE,
    states: [],
    components: [
        MapColorComponent { r: 151, g: 109, b: 77, a: 255 },
    ],
    permutations: [],
};
