use crate::block_definition::BlockDefinition;
use crate::block_id;
use crate::component::internal_friction_component::InternalFrictionComponent;
use crate::component::light_dampening_component::LightDampeningComponent;
use crate::component::light_emission_component::LightEmissionComponent;
use crate::component::map_color_component::MapColorComponent;
use crate::component::mineable_component::MineableComponent;
use crate::component::moveable_component::{MoveableComponent, Movement};
use crate::component::replaceable_component::ReplaceableComponent;
use crate::component::solid_component::SolidComponent;
use crate::component::transparent_component::TransparentComponent;
use crate::const_block;
use crate::state::common::AGE_16;

pub const SOUL_FIRE: BlockDefinition = const_block! {
    identifier: block_id::SOUL_FIRE,
    states: [AGE_16],
    components: [
        SolidComponent { solid: false },
        TransparentComponent { transparent: true },
        MapColorComponent { r: 102, g: 153, b: 216, a: 255 },
        InternalFrictionComponent { internal_friction: 0.95 },
        LightEmissionComponent { emission: 15 },
        LightDampeningComponent { dampening: 1 },
        ReplaceableComponent { replaceable: true },
        MineableComponent::hardness(0.0),
        MoveableComponent { movement: Movement::Break, sticky: false },
    ],
    permutations: [],
};
