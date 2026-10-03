use crate::block_definition::BlockDefinition;
use crate::block_id;
use crate::component::light_dampening_component::LightDampeningComponent;
use crate::component::map_color_component::MapColorComponent;
use crate::component::mineable_component::MineableComponent;
use crate::component::moveable_component::{MoveableComponent, Movement};
use crate::const_block;

pub const HONEY_BLOCK: BlockDefinition = const_block! {
    identifier: block_id::HONEY_BLOCK,
    states: [],
    components: [
        MapColorComponent { r: 216, g: 127, b: 51, a: 255 },
        LightDampeningComponent { dampening: 1 },
        MineableComponent::hardness(0.0),
        MoveableComponent { movement: Movement::Both, sticky: true },
    ],
    permutations: [],
};
