use crate::block_definition::BlockDefinition;
use crate::block_id;
use crate::component::collision_box_component::CollisionBoxComponent;
use crate::component::light_dampening_component::LightDampeningComponent;
use crate::component::mineable_component::MineableComponent;
use crate::component::transparent_component::TransparentComponent;
use crate::const_block;
use crate::state::common::PILLAR_AXIS;
use glam::Vec3;

pub const WEATHERED_COPPER_CHAIN: BlockDefinition = const_block! {
    identifier: block_id::WEATHERED_COPPER_CHAIN,
    states: [PILLAR_AXIS],
    components: [
        TransparentComponent { transparent: true },
        LightDampeningComponent { dampening: 1 },
        MineableComponent::hardness(5.0),
        CollisionBoxComponent::new(Vec3::new(0.4375, 0.0, 0.4375), Vec3::new(0.125, 1.0, 0.125)),
    ],
    permutations: [],
};
