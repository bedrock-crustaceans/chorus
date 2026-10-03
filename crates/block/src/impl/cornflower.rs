use crate::block_definition::BlockDefinition;
use crate::block_id;
use crate::component::collision_box_component::CollisionBoxComponent;
use crate::component::internal_friction_component::InternalFrictionComponent;
use crate::component::light_dampening_component::LightDampeningComponent;
use crate::component::map_color_component::MapColorComponent;
use crate::component::mineable_component::MineableComponent;
use crate::component::moveable_component::{MoveableComponent, Movement};
use crate::component::solid_component::SolidComponent;
use crate::component::transparent_component::TransparentComponent;
use crate::const_block;

pub const CORNFLOWER: BlockDefinition = const_block! {
    identifier: block_id::CORNFLOWER,
    states: [],
    components: [
        SolidComponent { solid: false },
        TransparentComponent { transparent: true },
        MapColorComponent { r: 0, g: 124, b: 0, a: 255 },
        InternalFrictionComponent { internal_friction: 0.95 },
        LightDampeningComponent { dampening: 1 },
        MineableComponent::hardness(0.0),
        MoveableComponent { movement: Movement::Break, sticky: false },
        CollisionBoxComponent::enabled(false),
    ],
    permutations: [],
};
