use crate::block::block_definition::BlockDefinition;
use crate::block::component::map_color_component::MapColorComponent;
use crate::block::component::mineable_component::MineableComponent;
use crate::block::state::common::HUGE_MUSHROOM_BITS;
use crate::const_block;

pub const RED_MUSHROOM_BLOCK: BlockDefinition = const_block! {
    identifier: "minecraft:red_mushroom_block",
    states: [HUGE_MUSHROOM_BITS],
    components: [
        MapColorComponent { r: 153, g: 51, b: 51, a: 255 },
        MineableComponent::hardness(0.2),
    ],
    permutations: [],
};
