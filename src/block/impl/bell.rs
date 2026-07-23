use crate::block::block_definition::BlockDefinition;
use crate::block::component::collision_box_component::CollisionBoxComponent;
use crate::block::component::light_dampening_component::LightDampeningComponent;
use crate::block::component::map_color_component::MapColorComponent;
use crate::block::component::mineable_component::MineableComponent;
use crate::block::component::transparent_component::TransparentComponent;
use crate::block::state::common::{ATTACHMENT, DIRECTION, TOGGLE_BIT};
use crate::{const_block, const_permutation};
use vek::Vec3;

pub const BELL: BlockDefinition = const_block! {
    identifier: "minecraft:bell",
    states: [ATTACHMENT, DIRECTION, TOGGLE_BIT],
    components: [
        TransparentComponent { transparent: true },
        MapColorComponent { r: 250, g: 238, b: 77, a: 255 },
        LightDampeningComponent { dampening: 1 },
        MineableComponent::hardness(1.0),
        CollisionBoxComponent::new(Vec3::new(0.249999, 0.249999, 0.249999), Vec3::new(0.5000020000000001, 0.750002, 0.5000020000000001)),
    ],
    permutations: [
        const_permutation! {
            condition: |it| it["attachment"] == "multiple",
            components: [CollisionBoxComponent::new(Vec3::new(0.249999, 0.249999, -1.0E-6), Vec3::new(0.5000020000000001, 0.5000020000000001, 1.0000019999999998))]
        },
        const_permutation! {
            condition: |it| (it["attachment"] == "multiple" && it["direction"] == 1) || (it["attachment"] == "multiple" && it["direction"] == 3),
            components: [CollisionBoxComponent::new(Vec3::new(-1.0E-6, 0.249999, 0.249999), Vec3::new(1.0000019999999998, 0.5000020000000001, 0.5000020000000001))]
        },
        const_permutation! {
            condition: |it| it["attachment"] == "side",
            components: [CollisionBoxComponent::new(Vec3::new(0.249999, 0.249999, -1.0E-6), Vec3::new(0.5000020000000001, 0.5000020000000001, 0.7500020000000001))]
        },
        const_permutation! {
            condition: |it| it["attachment"] == "side" && it["direction"] == 1,
            components: [CollisionBoxComponent::new(Vec3::new(0.249999, 0.249999, 0.249999), Vec3::new(0.750002, 0.5000020000000001, 0.5000020000000001))]
        },
        const_permutation! {
            condition: |it| it["attachment"] == "side" && it["direction"] == 2,
            components: [CollisionBoxComponent::new(Vec3::new(0.249999, 0.249999, 0.249999), Vec3::new(0.5000020000000001, 0.5000020000000001, 0.750002))]
        },
        const_permutation! {
            condition: |it| it["attachment"] == "side" && it["direction"] == 3,
            components: [CollisionBoxComponent::new(Vec3::new(-1.0E-6, 0.249999, 0.249999), Vec3::new(0.7500020000000001, 0.5000020000000001, 0.5000020000000001))]
        },
        const_permutation! {
            condition: |it| it["attachment"] == "standing",
            components: [CollisionBoxComponent::new(Vec3::new(-1.0E-6, -1.0E-6, 0.249999), Vec3::new(1.0000019999999998, 0.7500020000000001, 0.5000020000000001))]
        },
        const_permutation! {
            condition: |it| (it["attachment"] == "standing" && it["direction"] == 1) || (it["attachment"] == "standing" && it["direction"] == 3),
            components: [CollisionBoxComponent::new(Vec3::new(0.249999, -1.0E-6, -1.0E-6), Vec3::new(0.5000020000000001, 0.7500020000000001, 1.0000019999999998))]
        },
    ],
};
