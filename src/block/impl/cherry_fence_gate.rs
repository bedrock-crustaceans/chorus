use crate::block::block_definition::BlockDefinition;
use crate::block::component::collision_box_component::CollisionBoxComponent;
use crate::block::component::flammable_component::FlammableComponent;
use crate::block::component::light_dampening_component::LightDampeningComponent;
use crate::block::component::map_color_component::MapColorComponent;
use crate::block::component::mineable_component::MineableComponent;
use crate::block::component::transparent_component::TransparentComponent;
use crate::block::state::common::{IN_WALL_BIT, MINECRAFT_CARDINAL_DIRECTION, OPEN_BIT};
use crate::{const_block, const_permutation};
use glam::Vec3;

pub const CHERRY_FENCE_GATE: BlockDefinition = const_block! {
    identifier: "minecraft:cherry_fence_gate",
    states: [IN_WALL_BIT, MINECRAFT_CARDINAL_DIRECTION, OPEN_BIT],
    components: [
        TransparentComponent { transparent: true },
        MapColorComponent { r: 209, g: 177, b: 161, a: 255 },
        LightDampeningComponent { dampening: 1 },
        FlammableComponent { catch_chance: 5, destroy_chance: 20 },
        MineableComponent::hardness(2.0),
        CollisionBoxComponent::new(Vec3::new(0.0, 0.0, 0.375), Vec3::new(1.0, 1.0, 0.25)),
    ],
    permutations: [
        const_permutation! {
            condition: |it| (it["minecraft:cardinal_direction"] == "west") || (it["minecraft:cardinal_direction"] == "east"),
            components: [CollisionBoxComponent::new(Vec3::new(0.375, 0.0, 0.0), Vec3::new(0.25, 1.0, 1.0))]
        },
    ],
};
