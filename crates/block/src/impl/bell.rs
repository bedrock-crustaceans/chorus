use crate::block_definition::BlockDefinition;
use crate::block_id;
use crate::component::collision_box_component::CollisionBoxComponent;
use crate::component::light_dampening_component::LightDampeningComponent;
use crate::component::map_color_component::MapColorComponent;
use crate::component::mineable_component::MineableComponent;
use crate::component::transparent_component::TransparentComponent;
use crate::state::common::{ATTACHMENT, DIRECTION, TOGGLE_BIT};
use crate::{const_block, const_permutation};
use glam::Vec3;

pub const BELL: BlockDefinition = const_block! {
    identifier: block_id::BELL,
    states: [ATTACHMENT, DIRECTION, TOGGLE_BIT],
    components: [
        TransparentComponent { transparent: true },
        MapColorComponent { r: 250, g: 238, b: 77, a: 255 },
        LightDampeningComponent { dampening: 1 },
        MineableComponent::hardness(1.0),
        CollisionBoxComponent::new(Vec3::new(0.25, 0.25, 0.25), Vec3::new(0.5, 0.75, 0.5)),
    ],
    permutations: [
        const_permutation! {
            condition: |it| it["attachment"] == "multiple",
            components: [CollisionBoxComponent::new(Vec3::new(0.25, 0.25, 0.0), Vec3::new(0.5, 0.5, 1.0))]
        },
        const_permutation! {
            condition: |it| !(it["attachment"] != "multiple" || it["direction"] != 1 && it["direction"] != 3),
            components: [CollisionBoxComponent::new(Vec3::new(0.0, 0.25, 0.25), Vec3::new(1.0, 0.5, 0.5))]
        },
        const_permutation! {
            condition: |it| it["attachment"] == "side",
            components: [CollisionBoxComponent::new(Vec3::new(0.25, 0.25, 0.0), Vec3::new(0.5, 0.5, 0.75))]
        },
        const_permutation! {
            condition: |it| it["attachment"] == "side" && it["direction"] == 1,
            components: [CollisionBoxComponent::new(Vec3::new(0.25, 0.25, 0.25), Vec3::new(0.75, 0.5, 0.5))]
        },
        const_permutation! {
            condition: |it| it["attachment"] == "side" && it["direction"] == 2,
            components: [CollisionBoxComponent::new(Vec3::new(0.25, 0.25, 0.25), Vec3::new(0.5, 0.5, 0.75))]
        },
        const_permutation! {
            condition: |it| it["attachment"] == "side" && it["direction"] == 3,
            components: [CollisionBoxComponent::new(Vec3::new(0.0, 0.25, 0.25), Vec3::new(0.75, 0.5, 0.5))]
        },
        const_permutation! {
            condition: |it| it["attachment"] == "standing",
            components: [CollisionBoxComponent::new(Vec3::new(0.0, 0.0, 0.25), Vec3::new(1.0, 0.75, 0.5))]
        },
        const_permutation! {
            condition: |it| !(it["attachment"] != "standing" || it["direction"] != 1 && it["direction"] != 3),
            components: [CollisionBoxComponent::new(Vec3::new(0.25, 0.0, 0.0), Vec3::new(0.5, 0.75, 1.0))]
        },
    ],
};
