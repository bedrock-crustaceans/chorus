use crate::block_definition::BlockDefinition;
use crate::block_id;
use crate::component::map_color_component::MapColorComponent;
use crate::component::mineable_component::MineableComponent;
use crate::component::moveable_component::{MoveableComponent, Movement};
use crate::const_block;

pub const MOB_SPAWNER: BlockDefinition = const_block! {
    identifier: block_id::MOB_SPAWNER,
    states: [],
    components: [
        MapColorComponent { r: 112, g: 112, b: 112, a: 255 },
        MineableComponent::hardness(5.0),
        MoveableComponent { movement: Movement::None, sticky: false },
    ],
    permutations: [],
};
