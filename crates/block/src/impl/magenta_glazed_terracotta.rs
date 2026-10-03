use crate::block_definition::BlockDefinition;
use crate::block_id;
use crate::component::map_color_component::MapColorComponent;
use crate::component::mineable_component::MineableComponent;
use crate::component::moveable_component::{MoveableComponent, Movement};
use crate::const_block;
use crate::state::common::FACING_DIRECTION;

pub const MAGENTA_GLAZED_TERRACOTTA: BlockDefinition = const_block! {
    identifier: block_id::MAGENTA_GLAZED_TERRACOTTA,
    states: [FACING_DIRECTION],
    components: [
        MapColorComponent { r: 178, g: 76, b: 216, a: 255 },
        MineableComponent::hardness(1.4),
        MoveableComponent { movement: Movement::Both, sticky: false },
    ],
    permutations: [],
};
