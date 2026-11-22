use crate::block_definition::BlockDefinition;
use crate::block_id;
use crate::component::collision_box_component::CollisionBoxComponent;
use crate::component::internal_friction_component::InternalFrictionComponent;
use crate::component::light_dampening_component::LightDampeningComponent;
use crate::component::light_emission_component::LightEmissionComponent;
use crate::component::liquid_detection_component::LiquidDetectionComponent;
use crate::component::mineable_component::MineableComponent;
use crate::component::replaceable_component::ReplaceableComponent;
use crate::component::solid_component::SolidComponent;
use crate::component::support_component::SupportComponent;
use crate::component::transparent_component::TransparentComponent;
use crate::const_block;

pub static LIGHT_BLOCK_13: BlockDefinition = const_block! {
    identifier: block_id::LIGHT_BLOCK_13,
    states: [],
    components: [
        TransparentComponent { transparent: true },
        InternalFrictionComponent { internal_friction: 0.95 },
        LightEmissionComponent { emission: 13 },
        LightDampeningComponent { dampening: 1 },
        ReplaceableComponent { replaceable: true },
        MineableComponent::hardness(0.0),
        CollisionBoxComponent::enabled(false),
        LiquidDetectionComponent::BROKEN_BY_WATER,
        SupportComponent::ALL,
        SolidComponent { solid: false },
    ],
    permutations: [],
};
