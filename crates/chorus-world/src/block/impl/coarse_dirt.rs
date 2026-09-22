use crate::block::block_definition::BlockDefinition;
use crate::block::component::map_color_component::MapColorComponent;
use crate::block::component::mineable_component::MineableComponent;
use crate::const_block;

pub const COARSE_DIRT: BlockDefinition = const_block! {
    identifier: "minecraft:coarse_dirt",
    states: [],
    components: [
        MapColorComponent { r: 151, g: 109, b: 77, a: 255 },
        MineableComponent::hardness(0.6),
    ],
    permutations: [],
};
