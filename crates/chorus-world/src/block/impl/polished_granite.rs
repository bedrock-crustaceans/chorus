use crate::block::block_definition::BlockDefinition;
use crate::block::component::map_color_component::MapColorComponent;
use crate::block::component::mineable_component::MineableComponent;
use crate::const_block;

pub const POLISHED_GRANITE: BlockDefinition = const_block! {
    identifier: "minecraft:polished_granite",
    states: [],
    components: [
        MapColorComponent { r: 151, g: 109, b: 77, a: 255 },
        MineableComponent::hardness(1.5),
    ],
    permutations: [],
};
