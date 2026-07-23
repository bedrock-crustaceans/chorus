use crate::block::block_definition::BlockDefinition;
use crate::block::component::map_color_component::MapColorComponent;
use crate::const_block;

pub const INFO_UPDATE: BlockDefinition = const_block! {
    identifier: "minecraft:info_update",
    states: [],
    components: [
        MapColorComponent { r: 151, g: 109, b: 77, a: 255 },
    ],
    permutations: [],
};
