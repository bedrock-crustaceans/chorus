use crate::block::block_definition::BlockDefinition;
use crate::block::component::map_color_component::MapColorComponent;
use crate::block::component::mineable_component::MineableComponent;
use crate::const_block;

pub const REDSTONE_BLOCK: BlockDefinition = const_block! {
    identifier: "minecraft:redstone_block",
    states: [],
    components: [
        MapColorComponent { r: 255, g: 0, b: 0, a: 255 },
        MineableComponent::hardness(5.0),
    ],
    permutations: [],
};
