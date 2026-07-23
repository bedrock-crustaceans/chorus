use crate::block::block_definition::BlockDefinition;
use crate::block::component::collision_box_component::CollisionBoxComponent;
use crate::block::component::internal_friction_component::InternalFrictionComponent;
use crate::block::component::light_dampening_component::LightDampeningComponent;
use crate::block::component::mineable_component::MineableComponent;
use crate::block::component::solid_component::SolidComponent;
use crate::block::component::transparent_component::TransparentComponent;
use crate::block::state::common::{RAIL_DATA_BIT, RAIL_DIRECTION_6};
use crate::const_block;
use vek::Vec3;

pub const GOLDEN_RAIL: BlockDefinition = const_block! {
    identifier: "minecraft:golden_rail",
    states: [RAIL_DATA_BIT, RAIL_DIRECTION_6],
    components: [
        SolidComponent { solid: false },
        TransparentComponent { transparent: true },
        InternalFrictionComponent { internal_friction: 0.95 },
        LightDampeningComponent { dampening: 1 },
        MineableComponent::hardness(0.7),
        CollisionBoxComponent::new(Vec3::new(0.0, 0.0, 0.0), Vec3::new(1.0, 0.125, 1.0)),
    ],
    permutations: [],
};
