use crate::block_definition::BlockDefinition;
use crate::block_id;
use crate::component::collision_box_component::CollisionBoxComponent;
use crate::component::light_dampening_component::LightDampeningComponent;
use crate::component::map_color_component::MapColorComponent;
use crate::component::mineable_component::MineableComponent;
use crate::component::transparent_component::TransparentComponent;
use crate::state::common::{ATTACHMENT, DIRECTION};
use crate::{const_block, const_permutation};
use glam::Vec3;

pub const GRINDSTONE: BlockDefinition = const_block! {
    identifier: block_id::GRINDSTONE,
    states: [ATTACHMENT, DIRECTION],
    components: [
        TransparentComponent { transparent: true },
        MapColorComponent { r: 167, g: 167, b: 167, a: 255 },
        LightDampeningComponent { dampening: 1 },
        MineableComponent::hardness(2.0),
        CollisionBoxComponent::new(Vec3::new(0.125, 0.125, 0.125), Vec3::new(0.75, 0.875, 0.75)),
    ],
    permutations: [
        const_permutation! {
            condition: |it| (it["attachment"] == "multiple") || (it["attachment"] == "side"),
            components: [CollisionBoxComponent::new(Vec3::new(0.125, 0.125, 0.0), Vec3::new(0.75, 0.75, 0.875))]
        },
        const_permutation! {
            condition: |it| !(it["direction"] != 1 || it["attachment"] != "multiple" && it["attachment"] != "side"),
            components: [CollisionBoxComponent::new(Vec3::new(0.125, 0.125, 0.125), Vec3::new(0.875, 0.75, 0.75))]
        },
        const_permutation! {
            condition: |it| !(it["direction"] != 2 || it["attachment"] != "multiple" && it["attachment"] != "side"),
            components: [CollisionBoxComponent::new(Vec3::new(0.125, 0.125, 0.125), Vec3::new(0.75, 0.75, 0.875))]
        },
        const_permutation! {
            condition: |it| !(it["direction"] != 3 || it["attachment"] != "multiple" && it["attachment"] != "side"),
            components: [CollisionBoxComponent::new(Vec3::new(0.0, 0.125, 0.125), Vec3::new(0.875, 0.75, 0.75))]
        },
        const_permutation! {
            condition: |it| it["attachment"] == "standing",
            components: [CollisionBoxComponent::new(Vec3::new(0.125, 0.0, 0.125), Vec3::new(0.75, 0.875, 0.75))]
        },
    ],
};
