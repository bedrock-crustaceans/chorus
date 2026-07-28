use crate::block::block_definition::BlockDefinition;
use crate::block::component::collision_box_component::CollisionBoxComponent;
use crate::block::component::light_dampening_component::LightDampeningComponent;
use crate::block::component::map_color_component::MapColorComponent;
use crate::block::component::mineable_component::MineableComponent;
use crate::block::component::moveable_component::{MoveableComponent, Movement};
use crate::block::component::transparent_component::TransparentComponent;
use crate::block::state::common::{AGE_3, DIRECTION};
use crate::{const_block, const_permutation};
use vek::Vec3;

pub const COCOA: BlockDefinition = const_block! {
    identifier: "minecraft:cocoa",
    states: [AGE_3, DIRECTION],
    components: [
        TransparentComponent { transparent: true },
        MapColorComponent { r: 0, g: 124, b: 0, a: 255 },
        LightDampeningComponent { dampening: 1 },
        MineableComponent::hardness(0.2),
        MoveableComponent { movement: Movement::Break, sticky: false },
        CollisionBoxComponent::new(Vec3::new(0.375, 0.4375, 0.6875), Vec3::new(0.25, 0.3125, 0.25)),
    ],
    permutations: [
        const_permutation! {
            condition: |it| it["direction"] == 1,
            components: [CollisionBoxComponent::new(Vec3::new(0.0625, 0.4375, 0.375), Vec3::new(0.25, 0.3125, 0.25))]
        },
        const_permutation! {
            condition: |it| it["direction"] == 2,
            components: [CollisionBoxComponent::new(Vec3::new(0.375, 0.4375, 0.0625), Vec3::new(0.25, 0.3125, 0.25))]
        },
        const_permutation! {
            condition: |it| it["direction"] == 3,
            components: [CollisionBoxComponent::new(Vec3::new(0.6875, 0.4375, 0.375), Vec3::new(0.25, 0.3125, 0.25))]
        },
        const_permutation! {
            condition: |it| (it["age"] == 1) || (it["age"] == 2),
            components: [CollisionBoxComponent::new(Vec3::new(0.3125, 0.3125, 0.5625), Vec3::new(0.375, 0.4375, 0.375))]
        },
        const_permutation! {
            condition: |it| !(it["direction"] != 1 || it["age"] != 1 && it["age"] != 2),
            components: [CollisionBoxComponent::new(Vec3::new(0.0625, 0.3125, 0.3125), Vec3::new(0.375, 0.4375, 0.375))]
        },
        const_permutation! {
            condition: |it| !(it["direction"] != 2 || it["age"] != 1 && it["age"] != 2),
            components: [CollisionBoxComponent::new(Vec3::new(0.3125, 0.3125, 0.0625), Vec3::new(0.375, 0.4375, 0.375))]
        },
        const_permutation! {
            condition: |it| !(it["direction"] != 3 || it["age"] != 1 && it["age"] != 2),
            components: [CollisionBoxComponent::new(Vec3::new(0.5625, 0.3125, 0.3125), Vec3::new(0.375, 0.4375, 0.375))]
        },
    ],
};
