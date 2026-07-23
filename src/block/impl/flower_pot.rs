use crate::block::block_definition::BlockDefinition;
use crate::block::component::collision_box_component::CollisionBoxComponent;
use crate::block::component::light_dampening_component::LightDampeningComponent;
use crate::block::component::mineable_component::MineableComponent;
use crate::block::component::moveable_component::{MoveableComponent, Movement};
use crate::block::component::solid_component::SolidComponent;
use crate::block::component::transparent_component::TransparentComponent;
use crate::block::state::common::UPDATE_BIT;
use crate::const_block;
use vek::Vec3;

pub const FLOWER_POT: BlockDefinition = const_block! {
    identifier: "minecraft:flower_pot",
    states: [UPDATE_BIT],
    components: [
        SolidComponent { solid: false },
        TransparentComponent { transparent: true },
        LightDampeningComponent { dampening: 1 },
        MineableComponent::hardness(0.0),
        MoveableComponent { movement: Movement::Break, sticky: false },
        CollisionBoxComponent::new(Vec3::new(0.3125, 0.0, 0.3125), Vec3::new(0.375, 0.375, 0.375)),
    ],
    permutations: [],
};
