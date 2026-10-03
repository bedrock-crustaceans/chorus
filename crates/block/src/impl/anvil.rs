use crate::block_definition::BlockDefinition;
use crate::block_id;
use crate::component::collision_box_component::CollisionBoxComponent;
use crate::component::light_dampening_component::LightDampeningComponent;
use crate::component::map_color_component::MapColorComponent;
use crate::component::mineable_component::MineableComponent;
use crate::component::transparent_component::TransparentComponent;
use crate::state::common::MINECRAFT_CARDINAL_DIRECTION;
use crate::{const_block, const_permutation};
use glam::Vec3;

pub const ANVIL: BlockDefinition = const_block! {
    identifier: block_id::ANVIL,
    states: [MINECRAFT_CARDINAL_DIRECTION],
    components: [
        TransparentComponent { transparent: true },
        MapColorComponent { r: 167, g: 167, b: 167, a: 255 },
        LightDampeningComponent { dampening: 1 },
        MineableComponent::hardness(5.0),
        CollisionBoxComponent::new(Vec3::new(0.125, 0.0, 0.0), Vec3::new(0.75, 1.0, 1.0)),
    ],
    permutations: [
        const_permutation! {
            condition: |it| (it["minecraft:cardinal_direction"] == "west") || (it["minecraft:cardinal_direction"] == "east"),
            components: [CollisionBoxComponent::new(Vec3::new(0.0, 0.0, 0.125), Vec3::new(1.0, 1.0, 0.75))]
        },
    ],
};
