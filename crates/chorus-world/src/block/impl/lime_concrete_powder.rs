use crate::block::block_definition::BlockDefinition;
use crate::block::component::map_color_component::MapColorComponent;
use crate::block::component::mineable_component::MineableComponent;
use crate::const_block;

pub const LIME_CONCRETE_POWDER: BlockDefinition = const_block! {
    identifier: "minecraft:lime_concrete_powder",
    states: [],
    components: [
        MapColorComponent { r: 127, g: 204, b: 25, a: 255 },
        MineableComponent::hardness(0.5),
    ],
    permutations: [],
};
