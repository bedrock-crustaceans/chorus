use crate::block::block_definition::BlockDefinition;
use crate::block::component::map_color_component::MapColorComponent;
use crate::block::component::mineable_component::MineableComponent;
use crate::const_block;

pub const CYAN_CONCRETE_POWDER: BlockDefinition = const_block! {
    identifier: "minecraft:cyan_concrete_powder",
    states: [],
    components: [
        MapColorComponent { r: 76, g: 127, b: 153, a: 255 },
        MineableComponent::hardness(0.5),
    ],
    permutations: [],
};
