use crate::block_definition::BlockDefinition;
use crate::block_id;
use crate::component::map_color_component::MapColorComponent;
use crate::component::mineable_component::MineableComponent;
use crate::const_block;
use crate::state::common::{FACING_DIRECTION, TRIGGERED_BIT};

pub const DROPPER: BlockDefinition = const_block! {
    identifier: block_id::DROPPER,
    states: [FACING_DIRECTION, TRIGGERED_BIT],
    components: [
        MapColorComponent { r: 112, g: 112, b: 112, a: 255 },
        MineableComponent::hardness(3.5),
    ],
    permutations: [],
};
