use crate::block::block_definition::BlockDefinition;
use crate::block::component::collision_box_component::CollisionBoxComponent;
use crate::block::component::light_dampening_component::LightDampeningComponent;
use crate::block::component::light_emission_component::LightEmissionComponent;
use crate::block::component::mineable_component::MineableComponent;
use crate::block::component::transparent_component::TransparentComponent;
use crate::block::state::common::FACING_DIRECTION;
use crate::const_block;
use vek::Vec3;

pub const END_ROD: BlockDefinition = const_block! {
    identifier: "minecraft:end_rod",
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
