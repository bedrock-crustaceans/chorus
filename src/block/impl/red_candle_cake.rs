use crate::block::block_definition::BlockDefinition;
use crate::block::component::collision_box_component::CollisionBoxComponent;
use crate::block::component::light_dampening_component::LightDampeningComponent;
use crate::block::component::mineable_component::MineableComponent;
use crate::block::component::moveable_component::{MoveableComponent, Movement};
use crate::block::component::transparent_component::TransparentComponent;
use crate::block::state::common::LIT;
use crate::{const_block, const_permutation};
use glam::Vec3;

pub const RED_CANDLE_CAKE: BlockDefinition = const_block! {
    identifier: "minecraft:red_candle_cake",
    states: [LIT],
    components: [
        TransparentComponent { transparent: true },
        LightDampeningComponent { dampening: 1 },
        MineableComponent::hardness(0.5),
        MoveableComponent { movement: Movement::Break, sticky: false },
        CollisionBoxComponent::new(Vec3::new(0.1875, 0.0, 0.0625), Vec3::new(0.75, 0.5, 0.875)),
    ],
    permutations: [
        const_permutation! {
            condition: |it| it["lit"] == false,
            components: [CollisionBoxComponent::new(Vec3::new(0.0625, 0.0, 0.0625), Vec3::new(0.875, 0.5, 0.875))]
        },
    ],
};
