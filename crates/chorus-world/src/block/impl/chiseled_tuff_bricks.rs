use crate::block::block_definition::BlockDefinition;
use crate::block::component::map_color_component::MapColorComponent;
use crate::block::component::mineable_component::MineableComponent;
use crate::const_block;

pub const CHISELED_TUFF_BRICKS: BlockDefinition = const_block! {
    identifier: "minecraft:chiseled_tuff_bricks",
    states: [],
    components: [
        MapColorComponent { r: 57, g: 41, b: 35, a: 255 },
        MineableComponent::hardness(1.5),
    ],
    permutations: [],
};
