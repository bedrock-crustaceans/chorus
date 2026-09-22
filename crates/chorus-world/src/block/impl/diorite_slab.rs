use crate::block::block_definition::BlockDefinition;
use crate::block::component::collision_box_component::CollisionBoxComponent;
use crate::block::component::light_dampening_component::LightDampeningComponent;
use crate::block::component::map_color_component::MapColorComponent;
use crate::block::component::mineable_component::MineableComponent;
use crate::block::component::transparent_component::TransparentComponent;
use crate::block::state::common::MINECRAFT_VERTICAL_HALF;
use crate::{const_block, const_permutation};
use glam::Vec3;

pub const DIORITE_SLAB: BlockDefinition = const_block! {
    identifier: "minecraft:diorite_slab",
    states: [MINECRAFT_VERTICAL_HALF],
    components: [
        TransparentComponent { transparent: true },
        MapColorComponent { r: 255, g: 252, b: 245, a: 255 },
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
