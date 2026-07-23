use crate::block::block_definition::BlockDefinition;
use crate::block::component::collision_box_component::CollisionBoxComponent;
use crate::block::component::internal_friction_component::InternalFrictionComponent;
use crate::block::component::light_dampening_component::LightDampeningComponent;
use crate::block::component::mineable_component::MineableComponent;
use crate::block::component::moveable_component::{MoveableComponent, Movement};
use crate::block::component::transparent_component::TransparentComponent;
use crate::block::state::common::{FACING_DIRECTION, ITEM_FRAME_MAP_BIT, ITEM_FRAME_PHOTO_BIT};
use crate::{const_block, const_permutation};
use vek::Vec3;

pub const GLOW_FRAME: BlockDefinition = const_block! {
    identifier: "minecraft:glow_frame",
    states: [FACING_DIRECTION, ITEM_FRAME_MAP_BIT, ITEM_FRAME_PHOTO_BIT],
    components: [
        TransparentComponent { transparent: true },
        InternalFrictionComponent { internal_friction: 0.95 },
        LightDampeningComponent { dampening: 1 },
        MineableComponent::hardness(0.25),
        MoveableComponent { movement: Movement::Break, sticky: false },
        CollisionBoxComponent { origin: Vec3::new(0.125, 0.125, 0.125), size: Vec3::new(0.75, 0.75, 0.75), enabled: false },
    ],
    permutations: [
        const_permutation! {
            condition: |it| it["facing_direction"] == 1,
            components: [CollisionBoxComponent { origin: Vec3::new(0.0, 0.125, 0.125), size: Vec3::new(0.0625, 0.75, 0.75), enabled: false }]
        },
        const_permutation! {
            condition: |it| it["facing_direction"] == 3,
            components: [CollisionBoxComponent { origin: Vec3::new(0.125, 0.125, 0.0), size: Vec3::new(0.75, 0.75, 0.0625), enabled: false }]
        },
        const_permutation! {
            condition: |it| it["facing_direction"] == 5,
            components: [CollisionBoxComponent { origin: Vec3::new(0.125, 0.0, 0.125), size: Vec3::new(0.75, 0.0625, 0.75), enabled: false }]
        },
    ],
};
