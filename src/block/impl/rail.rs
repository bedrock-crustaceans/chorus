use crate::block::block_definition::BlockDefinition;
use crate::block::component::collision_box_component::CollisionBoxComponent;
use crate::block::component::internal_friction_component::InternalFrictionComponent;
use crate::block::component::light_dampening_component::LightDampeningComponent;
use crate::block::component::mineable_component::MineableComponent;
use crate::block::component::solid_component::SolidComponent;
use crate::block::component::transparent_component::TransparentComponent;
use crate::block::state::common::RAIL_DIRECTION_10;
use crate::const_block;
use glam::Vec3;

pub const RAIL: BlockDefinition = const_block! {
    identifier: "minecraft:rail",
    states: [RAIL_DIRECTION_10],
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
