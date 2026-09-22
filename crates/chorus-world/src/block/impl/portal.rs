use crate::block::block_definition::BlockDefinition;
use crate::block::component::internal_friction_component::InternalFrictionComponent;
use crate::block::component::light_dampening_component::LightDampeningComponent;
use crate::block::component::light_emission_component::LightEmissionComponent;
use crate::block::component::mineable_component::MineableComponent;
use crate::block::component::moveable_component::{MoveableComponent, Movement};
use crate::block::component::solid_component::SolidComponent;
use crate::block::component::transparent_component::TransparentComponent;
use crate::block::state::common::PORTAL_AXIS;
use crate::const_block;

pub const PORTAL: BlockDefinition = const_block! {
    identifier: "minecraft:portal",
    states: [PORTAL_AXIS],
    components: [
        SolidComponent { solid: false },
        TransparentComponent { transparent: true },
        InternalFrictionComponent { internal_friction: 0.95 },
        LightEmissionComponent { emission: 11 },
        LightDampeningComponent { dampening: 1 },
        MineableComponent::hardness(-1.0),
        MoveableComponent { movement: Movement::Break, sticky: false },
    ],
    permutations: [],
};
