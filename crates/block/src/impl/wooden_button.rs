use crate::block_definition::BlockDefinition;
use crate::block_id;
use crate::component::collision_box_component::CollisionBoxComponent;
use crate::component::internal_friction_component::InternalFrictionComponent;
use crate::component::light_dampening_component::LightDampeningComponent;
use crate::component::liquid_detection_component::LiquidDetectionComponent;
use crate::component::mineable_component::MineableComponent;
use crate::component::moveable_component::{MoveableComponent, Movement};
use crate::component::transparent_component::TransparentComponent;
use crate::const_block;
use crate::state::common::{BUTTON_PRESSED_BIT, FACING_DIRECTION};

pub static WOODEN_BUTTON: BlockDefinition = const_block! {
    identifier: block_id::WOODEN_BUTTON,
    states: [BUTTON_PRESSED_BIT, FACING_DIRECTION],
    components: [
        TransparentComponent { transparent: true },
        InternalFrictionComponent { internal_friction: 0.95 },
        LightDampeningComponent { dampening: 1 },
        MineableComponent::hardness(0.5),
        MoveableComponent { movement: Movement::Break, sticky: false },
        CollisionBoxComponent::enabled(false),
        LiquidDetectionComponent::CONTAINS_WATER,
    ],
    permutations: [],
};
