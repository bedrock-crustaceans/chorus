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
use crate::state::common::LIQUID_DEPTH;
use crate::{const_block, const_permutation};
use glam::Vec3;

pub const WATER: BlockDefinition = const_block! {
    identifier: block_id::WATER,
    states: [LIQUID_DEPTH],
    components: [
        SolidComponent { solid: false },
        TransparentComponent { transparent: true },
        InternalFrictionComponent { internal_friction: 0.5 },
        LightDampeningComponent { dampening: 2 },
        ReplaceableComponent { replaceable: true },
        MineableComponent::hardness(100.0),
        MoveableComponent { movement: Movement::Break, sticky: false },
        CollisionBoxComponent { origin: Vec3::new(0.0, 0.0, 0.0), size: Vec3::new(1.0, 0.888_888_9, 1.0), enabled: false },
    ],
    permutations: [
        const_permutation! {
            condition: |it| it["liquid_depth"] == 1,
            components: [CollisionBoxComponent { origin: Vec3::new(0.0, 0.0, 0.0), size: Vec3::new(1.0, 0.777_777_8, 1.0), enabled: false }]
        },
        const_permutation! {
            condition: |it| it["liquid_depth"] == 2,
            components: [CollisionBoxComponent { origin: Vec3::new(0.0, 0.0, 0.0), size: Vec3::new(1.0, 0.666_666_6, 1.0), enabled: false }]
        },
        const_permutation! {
            condition: |it| it["liquid_depth"] == 3,
            components: [CollisionBoxComponent { origin: Vec3::new(0.0, 0.0, 0.0), size: Vec3::new(1.0, 0.555_555_5, 1.0), enabled: false }]
        },
        const_permutation! {
            condition: |it| it["liquid_depth"] == 4,
            components: [CollisionBoxComponent { origin: Vec3::new(0.0, 0.0, 0.0), size: Vec3::new(1.0, 0.444_444_42, 1.0), enabled: false }]
        },
        const_permutation! {
            condition: |it| it["liquid_depth"] == 5,
            components: [CollisionBoxComponent { origin: Vec3::new(0.0, 0.0, 0.0), size: Vec3::new(1.0, 0.333_333_3, 1.0), enabled: false }]
        },
        const_permutation! {
            condition: |it| it["liquid_depth"] == 6,
            components: [CollisionBoxComponent { origin: Vec3::new(0.0, 0.0, 0.0), size: Vec3::new(1.0, 0.222_222_21, 1.0), enabled: false }]
        },
        const_permutation! {
            condition: |it| it["liquid_depth"] == 7,
            components: [CollisionBoxComponent { origin: Vec3::new(0.0, 0.0, 0.0), size: Vec3::new(1.0, 0.111_111_104, 1.0), enabled: false }]
        },
    ],
};
