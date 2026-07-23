use crate::block::block_definition::BlockDefinition;
use crate::block::component::collision_box_component::CollisionBoxComponent;
use crate::block::component::internal_friction_component::InternalFrictionComponent;
use crate::block::component::light_dampening_component::LightDampeningComponent;
use crate::block::component::light_emission_component::LightEmissionComponent;
use crate::block::component::mineable_component::MineableComponent;
use crate::block::component::replaceable_component::ReplaceableComponent;
use crate::block::component::transparent_component::TransparentComponent;
use crate::const_block;

pub const LIGHT_BLOCK_11: BlockDefinition = const_block! {
    identifier: "minecraft:light_block_11",
    states: [],
    components: [
        TransparentComponent { transparent: true },
        InternalFrictionComponent { internal_friction: 0.95 },
        LightEmissionComponent { emission: 11 },
        LightDampeningComponent { dampening: 1 },
        ReplaceableComponent { replaceable: true },
        MineableComponent::hardness(0.0),
        CollisionBoxComponent::enabled(false),
    ],
    permutations: [],
};
