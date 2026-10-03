use crate::block_definition::BlockDefinition;
use crate::block_id;
use crate::component::map_color_component::MapColorComponent;
use crate::component::mineable_component::MineableComponent;
use crate::component::moveable_component::{MoveableComponent, Movement};
use crate::const_block;
use crate::state::common::STRUCTURE_BLOCK_TYPE;

pub const STRUCTURE_BLOCK: BlockDefinition = const_block! {
    identifier: block_id::STRUCTURE_BLOCK,
    states: [STRUCTURE_BLOCK_TYPE],
    components: [
        MapColorComponent { r: 153, g: 153, b: 153, a: 255 },
        MineableComponent::hardness(-1.0),
        MoveableComponent { movement: Movement::None, sticky: false },
    ],
    permutations: [],
};
