use crate::block_definition::BlockDefinition;
use crate::block_id;
use crate::component::map_color_component::MapColorComponent;
use crate::const_block;

pub const RESERVED6: BlockDefinition = const_block! {
    identifier: block_id::RESERVED6,
    states: [],
    components: [
        MapColorComponent { r: 151, g: 109, b: 77, a: 255 },
    ],
    permutations: [],
};
