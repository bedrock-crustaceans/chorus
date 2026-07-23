use crate::block::block_definition::BlockDefinition;
use crate::block::component::collision_box_component::CollisionBoxComponent;
use crate::block::component::light_dampening_component::LightDampeningComponent;
use crate::block::component::mineable_component::MineableComponent;
use crate::block::component::moveable_component::{MoveableComponent, Movement};
use crate::block::component::transparent_component::TransparentComponent;
use crate::block::state::common::BITE_COUNTER;
use crate::const_block;
use vek::Vec3;

pub const CAKE: BlockDefinition = const_block! {
    identifier: "minecraft:cake",
    states: [BITE_COUNTER],
    components: [
        TransparentComponent { transparent: true },
        LightDampeningComponent { dampening: 1 },
        MineableComponent::hardness(0.5),
        MoveableComponent { movement: Movement::Break, sticky: false },
        CollisionBoxComponent::new(Vec3::new(0.0, 0.0, 0.0625), Vec3::new(0.9375, 0.5, 0.875)),
    ],
    permutations: [],
};
