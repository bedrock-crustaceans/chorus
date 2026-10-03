use crate::block_definition::BlockDefinition;
use crate::block_id;
use crate::component::map_color_component::MapColorComponent;
use crate::component::mineable_component::MineableComponent;
use crate::const_block;
use crate::state::common::HUGE_MUSHROOM_BITS;

pub const BROWN_MUSHROOM_BLOCK: BlockDefinition = const_block! {
    identifier: block_id::BROWN_MUSHROOM_BLOCK,
    states: [HUGE_MUSHROOM_BITS],
    components: [
        MapColorComponent { r: 151, g: 109, b: 77, a: 255 },
        MineableComponent::hardness(0.2),
    ],
    permutations: [],
};
