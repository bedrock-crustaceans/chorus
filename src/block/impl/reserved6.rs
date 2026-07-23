use crate::block::block_definition::BlockDefinition;
use crate::block::component::map_color_component::MapColorComponent;
use crate::const_block;

pub const RESERVED6: BlockDefinition = const_block! {
    identifier: "minecraft:reserved6",
    states: [],
    components: [
        MapColorComponent { r: 151, g: 109, b: 77, a: 255 },
    ],
    permutations: [],
};
