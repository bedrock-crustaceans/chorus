use crate::block::block_definition::BlockDefinition;
use crate::block::component::map_color_component::MapColorComponent;
use crate::block::component::mineable_component::MineableComponent;
use crate::const_block;

pub const BROWN_CONCRETE: BlockDefinition = const_block! {
    identifier: "minecraft:brown_concrete",
    states: [],
    components: [
        MapColorComponent { r: 102, g: 76, b: 51, a: 255 },
        MineableComponent::hardness(1.8),
    ],
    permutations: [],
};
