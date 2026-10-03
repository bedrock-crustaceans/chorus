use crate::block_definition::BlockDefinition;
use crate::block_id;
use crate::component::map_color_component::MapColorComponent;
use crate::component::mineable_component::MineableComponent;
use crate::component::moveable_component::{MoveableComponent, Movement};
use crate::const_block;
use crate::state::common::{FACING_DIRECTION, ROTATION};

pub const JIGSAW: BlockDefinition = const_block! {
    identifier: block_id::JIGSAW,
    states: [FACING_DIRECTION, ROTATION],
    components: [
        MapColorComponent { r: 153, g: 153, b: 153, a: 255 },
        MineableComponent::hardness(-1.0),
        MoveableComponent { movement: Movement::None, sticky: false },
    ],
    permutations: [],
};
