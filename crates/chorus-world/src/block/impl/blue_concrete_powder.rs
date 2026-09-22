use crate::block::block_definition::BlockDefinition;
use crate::block::component::map_color_component::MapColorComponent;
use crate::block::component::mineable_component::MineableComponent;
use crate::const_block;

pub const BLUE_CONCRETE_POWDER: BlockDefinition = const_block! {
    identifier: "minecraft:blue_concrete_powder",
    states: [],
    components: [
        MapColorComponent { r: 51, g: 76, b: 178, a: 255 },
        MineableComponent::hardness(0.5),
    ],
    permutations: [],
};
