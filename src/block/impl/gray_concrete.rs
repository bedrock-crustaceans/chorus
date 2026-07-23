use crate::block::block_definition::BlockDefinition;
use crate::block::component::map_color_component::MapColorComponent;
use crate::block::component::mineable_component::MineableComponent;
use crate::const_block;

pub const GRAY_CONCRETE: BlockDefinition = const_block! {
    identifier: "minecraft:gray_concrete",
    states: [],
    components: [
        MapColorComponent { r: 76, g: 76, b: 76, a: 255 },
        MineableComponent::hardness(1.8),
    ],
    permutations: [],
};
