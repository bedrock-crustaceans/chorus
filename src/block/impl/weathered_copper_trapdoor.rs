use crate::block::block_definition::BlockDefinition;
use crate::block::component::collision_box_component::CollisionBoxComponent;
use crate::block::component::light_dampening_component::LightDampeningComponent;
use crate::block::component::map_color_component::MapColorComponent;
use crate::block::component::mineable_component::MineableComponent;
use crate::block::component::transparent_component::TransparentComponent;
use crate::block::state::common::{DIRECTION, OPEN_BIT, UPSIDE_DOWN_BIT};
use crate::{const_block, const_permutation};
use glam::Vec3;

pub const WEATHERED_COPPER_TRAPDOOR: BlockDefinition = const_block! {
    identifier: "minecraft:weathered_copper_trapdoor",
    states: [DIRECTION, OPEN_BIT, UPSIDE_DOWN_BIT],
    components: [
        TransparentComponent { transparent: true },
        MapColorComponent { r: 167, g: 167, b: 167, a: 255 },
        LightDampeningComponent { dampening: 1 },
        MineableComponent::hardness(3.0),
        CollisionBoxComponent::new(Vec3::new(0.0, 0.0, 0.0), Vec3::new(0.1875, 1.0, 1.0)),
    ],
    permutations: [
        const_permutation! {
            condition: |it| it["open_bit"] == false,
            components: [CollisionBoxComponent::new(Vec3::new(0.0, 0.8125, 0.0), Vec3::new(1.0, 0.1875, 1.0))]
        },
        const_permutation! {
            condition: |it| it["open_bit"] == false && it["upside_down_bit"] == false,
            components: [CollisionBoxComponent::new(Vec3::new(0.0, 0.0, 0.0), Vec3::new(1.0, 0.1875, 1.0))]
        },
        const_permutation! {
            condition: |it| it["direction"] == 1,
            components: [CollisionBoxComponent::new(Vec3::new(0.8125, 0.0, 0.0), Vec3::new(0.1875, 1.0, 1.0))]
        },
        const_permutation! {
            condition: |it| it["direction"] == 2,
            components: [CollisionBoxComponent::new(Vec3::new(0.0, 0.0, 0.0), Vec3::new(1.0, 1.0, 0.1875))]
        },
        const_permutation! {
            condition: |it| it["direction"] == 3,
            components: [CollisionBoxComponent::new(Vec3::new(0.0, 0.0, 0.8125), Vec3::new(1.0, 1.0, 0.1875))]
        },
    ],
};
