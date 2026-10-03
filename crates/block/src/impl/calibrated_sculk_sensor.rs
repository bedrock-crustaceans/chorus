use crate::block_definition::BlockDefinition;
use crate::block_id;
use crate::component::light_dampening_component::LightDampeningComponent;
use crate::component::map_color_component::MapColorComponent;
use crate::component::mineable_component::MineableComponent;
use crate::component::moveable_component::{MoveableComponent, Movement};
use crate::component::solid_component::SolidComponent;
use crate::component::transparent_component::TransparentComponent;
use crate::const_block;
use crate::state::common::{MINECRAFT_CARDINAL_DIRECTION, SCULK_SENSOR_PHASE};

pub const CALIBRATED_SCULK_SENSOR: BlockDefinition = const_block! {
    identifier: block_id::CALIBRATED_SCULK_SENSOR,
    states: [MINECRAFT_CARDINAL_DIRECTION, SCULK_SENSOR_PHASE],
    components: [
        SolidComponent { solid: false },
        TransparentComponent { transparent: true },
        MapColorComponent { r: 13, g: 18, b: 23, a: 255 },
        LightDampeningComponent { dampening: 1 },
        MineableComponent::hardness(0.0),
        MoveableComponent { movement: Movement::Both, sticky: false },
    ],
    permutations: [],
};
