use crate::block::block_definition::BlockDefinition;
use crate::block::component::collision_box_component::CollisionBoxComponent;
use crate::block::component::internal_friction_component::InternalFrictionComponent;
use crate::block::component::light_dampening_component::LightDampeningComponent;
use crate::block::component::mineable_component::MineableComponent;
use crate::block::component::transparent_component::TransparentComponent;
use crate::block::state::common::{ATTACHED_BIT, DISARMED_BIT, POWERED_BIT, SUSPENDED_BIT};
use crate::const_block;
use vek::Vec3;

pub const TRIP_WIRE: BlockDefinition = const_block! {
    identifier: "minecraft:trip_wire",
    states: [ATTACHED_BIT, DISARMED_BIT, POWERED_BIT, SUSPENDED_BIT],
    components: [
        TransparentComponent { transparent: true },
        InternalFrictionComponent { internal_friction: 0.95 },
        LightDampeningComponent { dampening: 1 },
        MineableComponent::hardness(0.0),
        CollisionBoxComponent::new(Vec3::new(0.0, 0.0, 0.0), Vec3::new(1.0, 0.5, 1.0)),
    ],
    permutations: [],
};
