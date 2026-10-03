use crate::block_definition::BlockDefinition;
use crate::block_id;
use crate::component::collision_box_component::CollisionBoxComponent;
use crate::component::internal_friction_component::InternalFrictionComponent;
use crate::component::light_dampening_component::LightDampeningComponent;
use crate::component::light_emission_component::LightEmissionComponent;
use crate::component::mineable_component::MineableComponent;
use crate::component::replaceable_component::ReplaceableComponent;
use crate::component::transparent_component::TransparentComponent;
use crate::const_block;

pub const LIGHT_BLOCK_2: BlockDefinition = const_block! {
    identifier: block_id::LIGHT_BLOCK_2,
    states: [],
    components: [
        TransparentComponent { transparent: true },
        InternalFrictionComponent { internal_friction: 0.95 },
        LightEmissionComponent { emission: 2 },
        LightDampeningComponent { dampening: 1 },
        ReplaceableComponent { replaceable: true },
        MineableComponent::hardness(0.0),
        CollisionBoxComponent::enabled(false),
    ],
    permutations: [],
};
