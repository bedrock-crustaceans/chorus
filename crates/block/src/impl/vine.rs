use crate::block_definition::BlockDefinition;
use crate::block_id;
use crate::component::collision_box_component::CollisionBoxComponent;
use crate::component::internal_friction_component::InternalFrictionComponent;
use crate::component::light_dampening_component::LightDampeningComponent;
use crate::component::mineable_component::MineableComponent;
use crate::component::moveable_component::{MoveableComponent, Movement};
use crate::component::replaceable_component::ReplaceableComponent;
use crate::component::solid_component::SolidComponent;
use crate::component::transparent_component::TransparentComponent;
use crate::state::common::VINE_DIRECTION_BITS;
use crate::{const_block, const_permutation};
use glam::Vec3;

pub const VINE: BlockDefinition = const_block! {
    identifier: block_id::VINE,
    states: [VINE_DIRECTION_BITS],
    components: [
        SolidComponent { solid: false },
        TransparentComponent { transparent: true },
        InternalFrictionComponent { internal_friction: 0.95 },
        LightDampeningComponent { dampening: 1 },
        ReplaceableComponent { replaceable: true },
        MineableComponent::hardness(0.2),
        MoveableComponent { movement: Movement::Break, sticky: false },
        CollisionBoxComponent { origin: Vec3::new(0.0, 0.9375, 0.0), size: Vec3::new(1.0, 0.0625, 1.0), enabled: false },
    ],
    permutations: [
        const_permutation! {
            condition: |it| (it["vine_direction_bits"] == 1) || (it["vine_direction_bits"] == 5),
            components: [CollisionBoxComponent { origin: Vec3::new(0.0, 0.0, 0.9375), size: Vec3::new(1.0, 1.0, 0.0625), enabled: false }]
        },
        const_permutation! {
            condition: |it| (it["vine_direction_bits"] == 2) || (it["vine_direction_bits"] == 6),
            components: [CollisionBoxComponent { origin: Vec3::new(0.0, 0.0, 0.0), size: Vec3::new(0.0625, 1.0, 1.0), enabled: false }]
        },
        const_permutation! {
            condition: |it| (it["vine_direction_bits"] == 3) || (it["vine_direction_bits"] == 7) || (it["vine_direction_bits"] == 9) || (it["vine_direction_bits"] == 10) || (it["vine_direction_bits"] == 11) || (it["vine_direction_bits"] == 13) || (it["vine_direction_bits"] == 14) || (it["vine_direction_bits"] == 15),
            components: [CollisionBoxComponent { origin: Vec3::new(0.0, 0.0, 0.0), size: Vec3::new(1.0, 1.0, 1.0), enabled: false }]
        },
        const_permutation! {
            condition: |it| it["vine_direction_bits"] == 4,
            components: [CollisionBoxComponent { origin: Vec3::new(1.0, 1.0, 1.0), size: Vec3::new(-1.0, -1.0, -1.0), enabled: false }]
        },
        const_permutation! {
            condition: |it| (it["vine_direction_bits"] == 8) || (it["vine_direction_bits"] == 12),
            components: [CollisionBoxComponent { origin: Vec3::new(0.9375, 0.0, 0.0), size: Vec3::new(0.0625, 1.0, 1.0), enabled: false }]
        },
    ],
};
