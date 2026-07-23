use crate::block::block_definition::BlockDefinition;
use crate::block::component::collision_box_component::CollisionBoxComponent;
use crate::block::component::light_dampening_component::LightDampeningComponent;
use crate::block::component::mineable_component::MineableComponent;
use crate::block::component::transparent_component::TransparentComponent;
use crate::block::state::common::PILLAR_AXIS;
use crate::const_block;
use vek::Vec3;

pub const CHAIN: BlockDefinition = const_block! {
    identifier: "minecraft:chain",
    states: [PILLAR_AXIS],
    components: [
        TransparentComponent { transparent: true },
        LightDampeningComponent { dampening: 1 },
        MineableComponent::hardness(5.0),
        CollisionBoxComponent::new(Vec3::new(0.4375, 0.0, 0.4375), Vec3::new(0.125, 1.0, 0.125)),
    ],
    permutations: [],
};
