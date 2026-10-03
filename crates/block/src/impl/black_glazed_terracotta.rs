use crate::block_definition::BlockDefinition;
use crate::block_id;
use crate::component::map_color_component::MapColorComponent;
use crate::component::mineable_component::MineableComponent;
use crate::component::moveable_component::{MoveableComponent, Movement};
use crate::const_block;
use crate::state::common::FACING_DIRECTION;

pub const BLACK_GLAZED_TERRACOTTA: BlockDefinition = const_block! {
    identifier: block_id::BLACK_GLAZED_TERRACOTTA,
    states: [FACING_DIRECTION],
    components: [
        MapColorComponent { r: 25, g: 25, b: 25, a: 255 },
        MineableComponent::hardness(1.4),
        MoveableComponent { movement: Movement::Both, sticky: false },
    ],
    permutations: [],
};
