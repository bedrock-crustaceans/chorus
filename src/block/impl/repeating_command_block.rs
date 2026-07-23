use crate::block::block_definition::BlockDefinition;
use crate::block::component::map_color_component::MapColorComponent;
use crate::block::component::moveable_component::{MoveableComponent, Movement};
use crate::block::state::common::{CONDITIONAL_BIT, FACING_DIRECTION};
use crate::const_block;

pub const REPEATING_COMMAND_BLOCK: BlockDefinition = const_block! {
    identifier: "minecraft:repeating_command_block",
    states: [CONDITIONAL_BIT, FACING_DIRECTION],
    components: [
        MapColorComponent { r: 153, g: 90, b: 205, a: 255 },
        MoveableComponent { movement: Movement::None, sticky: false },
    ],
    permutations: [],
};
