use crate::block::block_definition::BlockDefinition;
use crate::block::component::collision_box_component::CollisionBoxComponent;
use crate::block::component::internal_friction_component::InternalFrictionComponent;
use crate::block::component::light_dampening_component::LightDampeningComponent;
use crate::block::component::map_color_component::MapColorComponent;
use crate::block::component::mineable_component::MineableComponent;
use crate::block::component::moveable_component::{MoveableComponent, Movement};
use crate::block::component::replaceable_component::ReplaceableComponent;
use crate::block::component::transparent_component::TransparentComponent;
use crate::block::state::common::{COVERED_BIT, HEIGHT};
use crate::{const_block, const_permutation};
use glam::Vec3;

pub const SNOW_LAYER: BlockDefinition = const_block! {
    identifier: "minecraft:snow_layer",
    states: [COVERED_BIT, HEIGHT],
    components: [
        TransparentComponent { transparent: true },
        MapColorComponent { r: 255, g: 255, b: 255, a: 255 },
        InternalFrictionComponent { internal_friction: 0.95 },
        LightDampeningComponent { dampening: 1 },
        ReplaceableComponent { replaceable: true },
        MineableComponent::hardness(0.2),
        MoveableComponent { movement: Movement::Break, sticky: false },
        CollisionBoxComponent { origin: Vec3::new(0.0, 0.0, 0.0), size: Vec3::new(1.0, 0.125, 1.0), enabled: false },
    ],
    permutations: [
        const_permutation! {
            condition: |it| it["height"] == 1,
            components: [CollisionBoxComponent { origin: Vec3::new(0.0, 0.0, 0.0), size: Vec3::new(1.0, 0.25, 1.0), enabled: false }]
        },
        const_permutation! {
            condition: |it| it["height"] == 2,
            components: [CollisionBoxComponent { origin: Vec3::new(0.0, 0.0, 0.0), size: Vec3::new(1.0, 0.375, 1.0), enabled: false }]
        },
        const_permutation! {
            condition: |it| (it["height"] == 3) || (it["height"] == 4) || (it["height"] == 5) || (it["height"] == 6) || (it["height"] == 7),
            components: [InternalFrictionComponent { internal_friction: 1.0 }]
        },
        const_permutation! {
            condition: |it| it["height"] == 3,
            components: [CollisionBoxComponent::new(Vec3::new(0.0, 0.0, 0.0), Vec3::new(1.0, 0.5, 1.0))]
        },
        const_permutation! {
            condition: |it| it["height"] == 4,
            components: [CollisionBoxComponent::new(Vec3::new(0.0, 0.0, 0.0), Vec3::new(1.0, 0.625, 1.0))]
        },
        const_permutation! {
            condition: |it| it["height"] == 5,
            components: [CollisionBoxComponent::new(Vec3::new(0.0, 0.0, 0.0), Vec3::new(1.0, 0.75, 1.0))]
        },
        const_permutation! {
            condition: |it| it["height"] == 6,
            components: [CollisionBoxComponent::new(Vec3::new(0.0, 0.0, 0.0), Vec3::new(1.0, 0.875, 1.0))]
        },
        const_permutation! {
            condition: |it| it["height"] == 7,
            components: [ReplaceableComponent { replaceable: false }]
        },
        const_permutation! {
            condition: |it| it["height"] == 7,
            components: [CollisionBoxComponent::new(Vec3::new(0.0, 0.0, 0.0), Vec3::new(1.0, 1.0, 1.0))]
        },
    ],
};
