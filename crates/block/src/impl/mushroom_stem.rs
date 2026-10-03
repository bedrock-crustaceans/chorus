use crate::block_definition::BlockDefinition;
use crate::block_id;
use crate::component::map_color_component::MapColorComponent;
use crate::component::mineable_component::MineableComponent;
use crate::const_block;
use crate::state::common::HUGE_MUSHROOM_BITS;

pub const MUSHROOM_STEM: BlockDefinition = const_block! {
    identifier: block_id::MUSHROOM_STEM,
    states: [HUGE_MUSHROOM_BITS],
    components: [
        MapColorComponent { r: 199, g: 199, b: 199, a: 255 },
        MineableComponent::hardness(0.2),
    ],
    permutations: [],
};
