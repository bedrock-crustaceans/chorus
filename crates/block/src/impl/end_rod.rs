use crate::block_definition::BlockDefinition;
use crate::block_id;
use crate::component::collision_box_component::CollisionBoxComponent;
use crate::component::light_dampening_component::LightDampeningComponent;
use crate::component::light_emission_component::LightEmissionComponent;
use crate::component::mineable_component::MineableComponent;
use crate::component::transparent_component::TransparentComponent;
use crate::const_block;
use crate::state::common::FACING_DIRECTION;
use glam::Vec3;

pub const END_ROD: BlockDefinition = const_block! {
    identifier: block_id::END_ROD,
    states: [FACING_DIRECTION],
    components: [
        TransparentComponent { transparent: true },
        LightEmissionComponent { emission: 14 },
        LightDampeningComponent { dampening: 1 },
        MineableComponent::hardness(0.0),
        CollisionBoxComponent::new(Vec3::new(0.4, 0.0, 0.4), Vec3::new(0.2, 1.0, 0.2)),
    ],
    permutations: [],
};
