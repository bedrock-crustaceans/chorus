use crate::block::block_definition::BlockDefinition;
use crate::block::block_id;
use crate::block::component::collision_box_component::CollisionBoxComponent;
use crate::block::component::flammable_component::FlammableComponent;
use crate::block::component::light_dampening_component::LightDampeningComponent;
use crate::block::component::map_color_component::MapColorComponent;
use crate::block::component::mineable_component::MineableComponent;
use crate::block::component::transparent_component::TransparentComponent;
use crate::block::state::common::MINECRAFT_VERTICAL_HALF;
use crate::{const_block, const_permutation};
use glam::Vec3;

pub const GRAY_WOOL_SLAB: BlockDefinition = const_block! {
    identifier: block_id::GRAY_WOOL_SLAB,
    states: [MINECRAFT_VERTICAL_HALF],
    components: [
        TransparentComponent { transparent: true },
        MapColorComponent { r: 76, g: 76, b: 76, a: 255 },
        FlammableComponent { catch_chance: 30, destroy_chance: 60 },
        LightDampeningComponent { dampening: 1 },
        MineableComponent::hardness(0.8),
        CollisionBoxComponent::new(Vec3::new(0.0, 0.0, 0.0), Vec3::new(1.0, 0.5, 1.0)),
    ],
    permutations: [
        const_permutation! {
            condition: |it| it["minecraft:vertical_half"] == "top",
            components: [CollisionBoxComponent::new(Vec3::new(0.0, 0.5, 0.0), Vec3::new(1.0, 0.5, 1.0))]
        },
    ],
};
