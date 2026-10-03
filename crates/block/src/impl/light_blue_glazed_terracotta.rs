use crate::block_definition::BlockDefinition;
use crate::block_id;
use crate::component::map_color_component::MapColorComponent;
use crate::component::mineable_component::MineableComponent;
use crate::component::moveable_component::{MoveableComponent, Movement};
use crate::const_block;
use crate::state::common::FACING_DIRECTION;

pub const LIGHT_BLUE_GLAZED_TERRACOTTA: BlockDefinition = const_block! {
    identifier: block_id::LIGHT_BLUE_GLAZED_TERRACOTTA,
    states: [FACING_DIRECTION],
    components: [
        MapColorComponent { r: 102, g: 153, b: 216, a: 255 },
        MineableComponent::hardness(1.4),
        MoveableComponent { movement: Movement::Both, sticky: false },
    ],
    permutations: [],
};
