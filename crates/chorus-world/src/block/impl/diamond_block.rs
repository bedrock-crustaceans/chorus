use crate::block::block_definition::BlockDefinition;
use crate::block::component::map_color_component::MapColorComponent;
use crate::block::component::mineable_component::MineableComponent;
use crate::const_block;

pub const DIAMOND_BLOCK: BlockDefinition = const_block! {
    identifier: "minecraft:diamond_block",
    states: [],
    components: [
        MapColorComponent { r: 92, g: 219, b: 213, a: 255 },
        MineableComponent::hardness(5.0),
    ],
    permutations: [],
};
