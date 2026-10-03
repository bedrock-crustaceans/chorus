use crate::block::block_definition::BlockDefinition;
use crate::block::block_id;
use crate::block::component::map_color_component::MapColorComponent;
use crate::block::component::mineable_component::MineableComponent;
use crate::block::state::common::HUGE_MUSHROOM_BITS;
use crate::const_block;

pub const BROWN_MUSHROOM_BLOCK: BlockDefinition = const_block! {
    identifier: block_id::BROWN_MUSHROOM_BLOCK,
    states: [HUGE_MUSHROOM_BITS],
    components: [
        MapColorComponent { r: 151, g: 109, b: 77, a: 255 },
        MineableComponent::hardness(0.2),
    ],
    permutations: [],
};
