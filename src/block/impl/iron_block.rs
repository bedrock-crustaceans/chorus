use crate::block::block_definition::BlockDefinition;
use crate::block::component::map_color_component::MapColorComponent;
use crate::block::component::mineable_component::MineableComponent;
use crate::const_block;

pub const IRON_BLOCK: BlockDefinition = const_block! {
    identifier: "minecraft:iron_block",
    states: [],
    components: [
        MapColorComponent { r: 167, g: 167, b: 167, a: 255 },
        MineableComponent::hardness(5.0),
    ],
    permutations: [],
};
