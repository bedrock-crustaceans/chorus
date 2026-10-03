use crate::block_definition::BlockDefinition;
use crate::block_id;
use crate::component::map_color_component::MapColorComponent;
use crate::const_block;

pub const INFO_UPDATE2: BlockDefinition = const_block! {
    identifier: block_id::INFO_UPDATE2,
    states: [],
    components: [
        MapColorComponent { r: 151, g: 109, b: 77, a: 255 },
    ],
    permutations: [],
};
