use crate::block::block_definition::BlockDefinition;
use crate::block::component::map_color_component::MapColorComponent;
use crate::block::component::mineable_component::MineableComponent;
use crate::const_block;

pub const RAW_GOLD_BLOCK: BlockDefinition = const_block! {
    identifier: "minecraft:raw_gold_block",
    states: [],
    components: [
        MapColorComponent { r: 250, g: 238, b: 77, a: 255 },
        MineableComponent::hardness(5.0),
    ],
    permutations: [],
};
