use crate::block::block_definition::BlockDefinition;
use crate::block::component::map_color_component::MapColorComponent;
use crate::const_block;

pub const REINFORCED_DEEPSLATE: BlockDefinition = const_block! {
    identifier: "minecraft:reinforced_deepslate",
    states: [],
    components: [
        MapColorComponent { r: 100, g: 100, b: 100, a: 255 },
    ],
    permutations: [],
};
