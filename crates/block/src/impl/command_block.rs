use crate::block_definition::BlockDefinition;
use crate::block_id;
use crate::component::map_color_component::MapColorComponent;
use crate::component::moveable_component::{MoveableComponent, Movement};
use crate::const_block;
use crate::state::common::{CONDITIONAL_BIT, FACING_DIRECTION};

pub const COMMAND_BLOCK: BlockDefinition = const_block! {
    identifier: block_id::COMMAND_BLOCK,
    states: [CONDITIONAL_BIT, FACING_DIRECTION],
    components: [
        MapColorComponent { r: 102, g: 76, b: 51, a: 255 },
        MoveableComponent { movement: Movement::None, sticky: false },
    ],
    permutations: [],
};
