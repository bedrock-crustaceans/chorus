use crate::block::block_definition::BlockDefinition;
use crate::block::component::map_color_component::MapColorComponent;
use crate::block::component::mineable_component::MineableComponent;
use crate::const_block;

pub const SMOOTH_SANDSTONE: BlockDefinition = const_block! {
    identifier: "minecraft:smooth_sandstone",
    states: [],
    components: [
        MapColorComponent { r: 247, g: 233, b: 163, a: 255 },
        MineableComponent::hardness(2.0),
    ],
    permutations: [],
};
