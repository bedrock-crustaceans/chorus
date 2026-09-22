use crate::block::block_definition::BlockDefinition;
use crate::block::component::collision_box_component::CollisionBoxComponent;
use crate::block::component::light_dampening_component::LightDampeningComponent;
use crate::block::component::mineable_component::MineableComponent;
use crate::block::component::moveable_component::{MoveableComponent, Movement};
use crate::block::component::solid_component::SolidComponent;
use crate::block::component::transparent_component::TransparentComponent;
use crate::block::state::common::FACING_DIRECTION;
use crate::{const_block, const_permutation};
use glam::Vec3;

pub const DRAGON_HEAD: BlockDefinition = const_block! {
    identifier: "minecraft:dragon_head",
    states: [FACING_DIRECTION],
    components: [
        SolidComponent { solid: false },
        TransparentComponent { transparent: true },
        LightDampeningComponent { dampening: 1 },
        MineableComponent::hardness(1.0),
        MoveableComponent { movement: Movement::Break, sticky: false },
        CollisionBoxComponent::new(Vec3::new(0.25, 0.0, 0.25), Vec3::new(0.5, 0.5, 0.5)),
    ],
    permutations: [
        const_permutation! {
            condition: |it| it["facing_direction"] == 2,
            components: [CollisionBoxComponent::new(Vec3::new(0.25, 0.25, 0.5), Vec3::new(0.5, 0.5, 0.5))]
        },
        const_permutation! {
            condition: |it| it["facing_direction"] == 3,
            components: [CollisionBoxComponent::new(Vec3::new(0.25, 0.25, 0.0), Vec3::new(0.5, 0.5, 0.5))]
        },
        const_permutation! {
            condition: |it| it["facing_direction"] == 4,
            components: [CollisionBoxComponent::new(Vec3::new(0.5, 0.25, 0.25), Vec3::new(0.5, 0.5, 0.5))]
        },
        const_permutation! {
            condition: |it| it["facing_direction"] == 5,
            components: [CollisionBoxComponent::new(Vec3::new(0.0, 0.25, 0.25), Vec3::new(0.5, 0.5, 0.5))]
        },
    ],
};
