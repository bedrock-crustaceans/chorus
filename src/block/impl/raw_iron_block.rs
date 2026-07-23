use crate::block::block_definition::BlockDefinition;
use crate::block::component::map_color_component::MapColorComponent;
use crate::const_block;

pub const RAW_IRON_BLOCK: BlockDefinition = const_block! {
    identifier: "minecraft:raw_iron_block",
    states: [],
    components: [
        MapColorComponent { r: 216, g: 175, b: 147, a: 255 },
    ],
    permutations: [],
};
