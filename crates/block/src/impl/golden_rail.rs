use crate::block_definition::BlockDefinition;
use crate::block_id;
use crate::component::collision_box_component::CollisionBoxComponent;
use crate::component::internal_friction_component::InternalFrictionComponent;
use crate::component::light_dampening_component::LightDampeningComponent;
use crate::component::mineable_component::MineableComponent;
use crate::component::solid_component::SolidComponent;
use crate::component::transparent_component::TransparentComponent;
use crate::const_block;
use crate::state::common::{RAIL_DATA_BIT, RAIL_DIRECTION_6};
use glam::Vec3;

pub const GOLDEN_RAIL: BlockDefinition = const_block! {
    identifier: block_id::GOLDEN_RAIL,
    states: [RAIL_DATA_BIT, RAIL_DIRECTION_6],
    components: [
        SolidComponent { solid: false },
        TransparentComponent { transparent: true },
        InternalFrictionComponent { internal_friction: 0.95 },
        LightDampeningComponent { dampening: 1 },
        MineableComponent::hardness(0.7),
        CollisionBoxComponent { origin: Vec3::new(0.0, 0.0, 0.0), size: Vec3::new(1.0, 0.125, 1.0), enabled: false },
    ],
    permutations: [],
};
