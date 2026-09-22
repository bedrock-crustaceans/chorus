use crate::block::block_definition::BlockDefinition;
use crate::block::component::map_color_component::MapColorComponent;
use crate::block::component::mineable_component::MineableComponent;
use crate::const_block;

pub const RAW_COPPER_BLOCK: BlockDefinition = const_block! {
    identifier: "minecraft:raw_copper_block",
    states: [],
    components: [
        MapColorComponent { r: 216, g: 127, b: 51, a: 255 },
        MineableComponent::hardness(5.0),
    ],
    permutations: [],
};
