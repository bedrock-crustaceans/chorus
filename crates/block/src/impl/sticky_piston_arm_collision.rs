use crate::block_definition::BlockDefinition;
use crate::block_id;
use crate::component::light_dampening_component::LightDampeningComponent;
use crate::component::map_color_component::MapColorComponent;
use crate::component::mineable_component::MineableComponent;
use crate::component::moveable_component::{MoveableComponent, Movement};
use crate::component::solid_component::SolidComponent;
use crate::component::transparent_component::TransparentComponent;
use crate::const_block;
use crate::state::common::FACING_DIRECTION;

pub const STICKY_PISTON_ARM_COLLISION: BlockDefinition = const_block! {
    identifier: block_id::STICKY_PISTON_ARM_COLLISION,
    states: [FACING_DIRECTION],
    components: [
        SolidComponent { solid: false },
        TransparentComponent { transparent: true },
        MapColorComponent { r: 112, g: 112, b: 112, a: 255 },
        LightDampeningComponent { dampening: 1 },
        MineableComponent::hardness(1.5),
        MoveableComponent { movement: Movement::None, sticky: false },
    ],
    permutations: [],
};
