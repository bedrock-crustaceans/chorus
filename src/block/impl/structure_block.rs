use crate::block::block_definition::BlockDefinition;
use crate::block::block_id;
use crate::block::component::map_color_component::MapColorComponent;
use crate::block::component::mineable_component::MineableComponent;
use crate::block::component::moveable_component::{MoveableComponent, Movement};
use crate::block::state::common::STRUCTURE_BLOCK_TYPE;
use crate::const_block;

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
