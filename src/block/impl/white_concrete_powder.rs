use crate::block::block_definition::BlockDefinition;
use crate::block::component::map_color_component::MapColorComponent;
use crate::block::component::mineable_component::MineableComponent;
use crate::const_block;

pub const WHITE_CONCRETE_POWDER: BlockDefinition = const_block! {
    identifier: "minecraft:white_concrete_powder",
    states: [],
    components: [
        MapColorComponent { r: 255, g: 255, b: 255, a: 255 },
        MineableComponent::hardness(0.5),
    ],
    permutations: [],
};
