use crate::block::block_definition::BlockDefinition;
use crate::block::component::collision_box_component::CollisionBoxComponent;
use crate::block::component::internal_friction_component::InternalFrictionComponent;
use crate::block::component::light_dampening_component::LightDampeningComponent;
use crate::block::component::mineable_component::MineableComponent;
use crate::block::component::moveable_component::{MoveableComponent, Movement};
use crate::block::component::replaceable_component::ReplaceableComponent;
use crate::block::component::solid_component::SolidComponent;
use crate::block::component::transparent_component::TransparentComponent;
use crate::block::state::common::VINE_DIRECTION_BITS;
use crate::{const_block, const_permutation};
use vek::Vec3;

pub const VINE: BlockDefinition = const_block! {
    identifier: "minecraft:vine",
    states: [VINE_DIRECTION_BITS],
    components: [
        SolidComponent { solid: false },
        TransparentComponent { transparent: true },
        InternalFrictionComponent { internal_friction: 0.95 },
        LightDampeningComponent { dampening: 1 },
        ReplaceableComponent { replaceable: true },
        MineableComponent::hardness(0.2),
        MoveableComponent { movement: Movement::Break, sticky: false },
        CollisionBoxComponent::new(Vec3::new(0.0, 0.9375, 0.0), Vec3::new(1.0, 0.0625, 1.0)),
    ],
    permutations: [
        const_permutation! {
            condition: |it| (it["vine_direction_bits"] == 1) || (it["vine_direction_bits"] == 5),
            components: [CollisionBoxComponent::new(Vec3::new(0.0, 0.0, 0.9375), Vec3::new(1.0, 1.0, 0.0625))]
        },
        const_permutation! {
            condition: |it| (it["vine_direction_bits"] == 2) || (it["vine_direction_bits"] == 6),
            components: [CollisionBoxComponent::new(Vec3::new(0.0, 0.0, 0.0), Vec3::new(0.0625, 1.0, 1.0))]
        },
        const_permutation! {
            condition: |it| (it["vine_direction_bits"] == 3) || (it["vine_direction_bits"] == 7) || (it["vine_direction_bits"] == 9) || (it["vine_direction_bits"] == 10) || (it["vine_direction_bits"] == 11) || (it["vine_direction_bits"] == 13) || (it["vine_direction_bits"] == 14) || (it["vine_direction_bits"] == 15),
            components: [CollisionBoxComponent::new(Vec3::new(0.0, 0.0, 0.0), Vec3::new(1.0, 1.0, 1.0))]
        },
        const_permutation! {
            condition: |it| it["vine_direction_bits"] == 4,
            components: [CollisionBoxComponent::new(Vec3::new(1.0, 1.0, 1.0), Vec3::new(-1.0, -1.0, -1.0))]
        },
        const_permutation! {
            condition: |it| (it["vine_direction_bits"] == 8) || (it["vine_direction_bits"] == 12),
            components: [CollisionBoxComponent::new(Vec3::new(0.9375, 0.0, 0.0), Vec3::new(0.0625, 1.0, 1.0))]
        },
    ],
};
