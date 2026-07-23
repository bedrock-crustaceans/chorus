use crate::block::block_definition::BlockDefinition;
use crate::block::component::map_color_component::MapColorComponent;
use crate::block::component::mineable_component::MineableComponent;
use crate::block::component::moveable_component::{MoveableComponent, Movement};
use crate::block::state::common::{FACING_DIRECTION, ROTATION};
use crate::const_block;

pub const JIGSAW: BlockDefinition = const_block! {
    identifier: "minecraft:jigsaw",
    states: [FACING_DIRECTION, ROTATION],
    components: [
        MapColorComponent { r: 153, g: 153, b: 153, a: 255 },
        MineableComponent::hardness(-1.0),
        MoveableComponent { movement: Movement::None, sticky: false },
    ],
    permutations: [],
};
