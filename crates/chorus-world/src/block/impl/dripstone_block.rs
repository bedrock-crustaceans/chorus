use crate::block::block_definition::BlockDefinition;
use crate::block::component::map_color_component::MapColorComponent;
use crate::block::component::mineable_component::MineableComponent;
use crate::const_block;

pub const DRIPSTONE_BLOCK: BlockDefinition = const_block! {
    identifier: "minecraft:dripstone_block",
    states: [],
    components: [
        MapColorComponent { r: 76, g: 50, b: 35, a: 255 },
        MineableComponent::hardness(1.5),
    ],
    permutations: [],
};
