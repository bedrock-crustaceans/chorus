use crate::block::block_definition::BlockDefinition;
use crate::block::component::map_color_component::MapColorComponent;
use crate::const_block;

pub const NETHERREACTOR: BlockDefinition = const_block! {
    identifier: "minecraft:netherreactor",
    states: [],
    components: [
        MapColorComponent { r: 167, g: 167, b: 167, a: 255 },
    ],
    permutations: [],
};
