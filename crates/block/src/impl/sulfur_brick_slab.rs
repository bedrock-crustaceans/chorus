use crate::block_definition::BlockDefinition;
use crate::block_id;
use crate::component::collision_box_component::CollisionBoxComponent;
use crate::component::light_dampening_component::LightDampeningComponent;
use crate::component::map_color_component::MapColorComponent;
use crate::component::mineable_component::MineableComponent;
use crate::component::transparent_component::TransparentComponent;
use crate::state::common::MINECRAFT_VERTICAL_HALF;
use crate::{const_block, const_permutation};
use glam::Vec3;

pub const SULFUR_BRICK_SLAB: BlockDefinition = const_block! {
    identifier: block_id::SULFUR_BRICK_SLAB,
    states: [MINECRAFT_VERTICAL_HALF],
    components: [
        TransparentComponent { transparent: true },
        MapColorComponent { r: 229, g: 229, b: 51, a: 255 },
        LightDampeningComponent { dampening: 1 },
        MineableComponent::hardness(1.5),
        CollisionBoxComponent::new(Vec3::new(0.0, 0.0, 0.0), Vec3::new(1.0, 0.5, 1.0)),
    ],
    permutations: [
        const_permutation! {
            condition: |it| it["minecraft:vertical_half"] == "top",
            components: [CollisionBoxComponent::new(Vec3::new(0.0, 0.5, 0.0), Vec3::new(1.0, 0.5, 1.0))]
        },
    ],
};
