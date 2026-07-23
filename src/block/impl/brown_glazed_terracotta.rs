use crate::block::block_definition::BlockDefinition;
use crate::block::component::map_color_component::MapColorComponent;
use crate::block::component::mineable_component::MineableComponent;
use crate::block::component::moveable_component::{MoveableComponent, Movement};
use crate::block::state::common::FACING_DIRECTION;
use crate::const_block;

pub const BROWN_GLAZED_TERRACOTTA: BlockDefinition = const_block! {
    identifier: "minecraft:brown_glazed_terracotta",
    states: [FACING_DIRECTION],
    components: [
        MapColorComponent { r: 102, g: 76, b: 51, a: 255 },
        MineableComponent::hardness(1.4),
        MoveableComponent { movement: Movement::Both, sticky: false },
    ],
    permutations: [],
};
