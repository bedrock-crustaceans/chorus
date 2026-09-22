use crate::block::block_definition::BlockDefinition;
use crate::block::component::map_color_component::MapColorComponent;
use crate::block::component::mineable_component::MineableComponent;
use crate::const_block;

pub const SMOOTH_BASALT: BlockDefinition = const_block! {
    identifier: "minecraft:smooth_basalt",
    states: [],
    components: [
        MapColorComponent { r: 25, g: 25, b: 25, a: 255 },
        MineableComponent::hardness(1.25),
    ],
    permutations: [],
};
