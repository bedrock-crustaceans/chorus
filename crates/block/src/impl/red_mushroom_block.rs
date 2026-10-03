use crate::block_definition::BlockDefinition;
use crate::block_id;
use crate::component::map_color_component::MapColorComponent;
use crate::component::mineable_component::MineableComponent;
use crate::const_block;
use crate::state::common::HUGE_MUSHROOM_BITS;

pub const RED_MUSHROOM_BLOCK: BlockDefinition = const_block! {
    identifier: block_id::RED_MUSHROOM_BLOCK,
    states: [HUGE_MUSHROOM_BITS],
    components: [
        MapColorComponent { r: 153, g: 51, b: 51, a: 255 },
        MineableComponent::hardness(0.2),
    ],
    permutations: [],
};
