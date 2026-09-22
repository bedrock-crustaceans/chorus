use crate::block::block_definition::BlockDefinition;
use crate::block::component::collision_box_component::CollisionBoxComponent;
use crate::block::component::light_dampening_component::LightDampeningComponent;
use crate::block::component::map_color_component::MapColorComponent;
use crate::block::component::mineable_component::MineableComponent;
use crate::block::component::moveable_component::{MoveableComponent, Movement};
use crate::block::component::solid_component::SolidComponent;
use crate::block::component::transparent_component::TransparentComponent;
use crate::block::state::common::{DOOR_HINGE_BIT, MINECRAFT_CARDINAL_DIRECTION, OPEN_BIT, UPPER_BLOCK_BIT};
use crate::{const_block, const_permutation};
use glam::Vec3;

pub const EXPOSED_COPPER_DOOR: BlockDefinition = const_block! {
    identifier: "minecraft:exposed_copper_door",
    states: [DOOR_HINGE_BIT, MINECRAFT_CARDINAL_DIRECTION, OPEN_BIT, UPPER_BLOCK_BIT],
    components: [
        SolidComponent { solid: false },
        TransparentComponent { transparent: true },
        MapColorComponent { r: 135, g: 107, b: 98, a: 255 },
        LightDampeningComponent { dampening: 1 },
        MineableComponent::hardness(3.0),
        MoveableComponent { movement: Movement::Break, sticky: false },
        CollisionBoxComponent::new(Vec3::new(0.0, 0.0, 0.8125), Vec3::new(1.0, 1.0, 0.1875)),
    ],
    permutations: [
        const_permutation! {
            condition: |it| (it["door_hinge_bit"] == false) || (it["minecraft:cardinal_direction"] == "north") || (it["minecraft:cardinal_direction"] == "west" && it["open_bit"] == false),
            components: [CollisionBoxComponent::new(Vec3::new(0.0, 0.0, 0.0), Vec3::new(1.0, 1.0, 0.1875))]
        },
        const_permutation! {
            condition: |it| (it["open_bit"] == false) || (it["minecraft:cardinal_direction"] == "west") || (it["minecraft:cardinal_direction"] == "east" && it["door_hinge_bit"] == false),
            components: [CollisionBoxComponent::new(Vec3::new(0.0, 0.0, 0.0), Vec3::new(0.1875, 1.0, 1.0))]
        },
        const_permutation! {
            condition: |it| (it["minecraft:cardinal_direction"] == "east") || (it["minecraft:cardinal_direction"] == "west" && it["door_hinge_bit"] == false) || (it["minecraft:cardinal_direction"] == "north" && it["open_bit"] == false),
            components: [CollisionBoxComponent::new(Vec3::new(0.8125, 0.0, 0.0), Vec3::new(0.1875, 1.0, 1.0))]
        },
    ],
};
