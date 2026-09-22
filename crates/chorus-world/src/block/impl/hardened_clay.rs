use crate::block::block_definition::BlockDefinition;
use crate::block::component::map_color_component::MapColorComponent;
use crate::block::component::mineable_component::MineableComponent;
use crate::const_block;

pub const HARDENED_CLAY: BlockDefinition = const_block! {
    identifier: "minecraft:hardened_clay",
    states: [],
    components: [
        MapColorComponent { r: 216, g: 127, b: 51, a: 255 },
        MineableComponent::hardness(1.25),
        MapColorComponent { r: 135, g: 107, b: 98, a: 255 },
        MineableComponent::hardness(1.25),
    ],
    permutations: [],
};
