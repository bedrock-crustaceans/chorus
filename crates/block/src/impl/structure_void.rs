use crate::block_definition::BlockDefinition;
use crate::block_id;
use crate::component::collision_box_component::CollisionBoxComponent;
use crate::component::internal_friction_component::InternalFrictionComponent;
use crate::component::liquid_detection_component::LiquidDetectionComponent;
use crate::component::mineable_component::MineableComponent;
use crate::component::moveable_component::{MoveableComponent, Movement};
use crate::component::solid_component::SolidComponent;
use crate::component::support_component::SupportComponent;
use crate::const_block;

pub static STRUCTURE_VOID: BlockDefinition = const_block! {
    identifier: block_id::STRUCTURE_VOID,
    states: [],
    components: [
        InternalFrictionComponent { internal_friction: 0.95 },
        MineableComponent::hardness(0.0),
        MoveableComponent { movement: Movement::None, sticky: false },
        LiquidDetectionComponent::BROKEN_BY_LAVA,
        CollisionBoxComponent::enabled(false),
        SupportComponent::ALL,
        SolidComponent { solid: false },
    ],
    permutations: [],
};
