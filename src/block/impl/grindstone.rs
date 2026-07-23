use crate::block::block_definition::BlockDefinition;
use crate::block::component::collision_box_component::CollisionBoxComponent;
use crate::block::component::light_dampening_component::LightDampeningComponent;
use crate::block::component::map_color_component::MapColorComponent;
use crate::block::component::mineable_component::MineableComponent;
use crate::block::component::transparent_component::TransparentComponent;
use crate::block::state::common::{ATTACHMENT, DIRECTION};
use crate::{const_block, const_permutation};
use vek::Vec3;

pub const GRINDSTONE: BlockDefinition = const_block! {
    identifier: "minecraft:grindstone",
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
            condition: |it| (it["attachment"] == "multiple" && it["direction"] == 1) || (it["attachment"] == "side" && it["direction"] == 1),
            components: [CollisionBoxComponent::new(Vec3::new(0.125, 0.125, 0.125), Vec3::new(0.875, 0.75, 0.75))]
        },
        const_permutation! {
            condition: |it| (it["attachment"] == "multiple" && it["direction"] == 2) || (it["attachment"] == "side" && it["direction"] == 2),
            components: [CollisionBoxComponent::new(Vec3::new(0.125, 0.125, 0.125), Vec3::new(0.75, 0.75, 0.875))]
        },
        const_permutation! {
            condition: |it| (it["attachment"] == "multiple" && it["direction"] == 3) || (it["attachment"] == "side" && it["direction"] == 3),
            components: [CollisionBoxComponent::new(Vec3::new(0.0, 0.125, 0.125), Vec3::new(0.875, 0.75, 0.75))]
        },
        const_permutation! {
            condition: |it| it["attachment"] == "standing",
            components: [CollisionBoxComponent::new(Vec3::new(0.125, 0.0, 0.125), Vec3::new(0.75, 0.875, 0.75))]
        },
    ],
};
