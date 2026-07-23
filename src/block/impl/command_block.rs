use crate::block::block_definition::BlockDefinition;
use crate::block::component::map_color_component::MapColorComponent;
use crate::block::component::moveable_component::{MoveableComponent, Movement};
use crate::block::state::common::{CONDITIONAL_BIT, FACING_DIRECTION};
use crate::const_block;

pub const COMMAND_BLOCK: BlockDefinition = const_block! {
    identifier: "minecraft:command_block",
    states: [CONDITIONAL_BIT, FACING_DIRECTION],
    components: [
        MapColorComponent { r: 102, g: 76, b: 51, a: 255 },
        MoveableComponent { movement: Movement::None, sticky: false },
    ],
    permutations: [],
};
