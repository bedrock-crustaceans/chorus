use crate::block_definition::BlockDefinition;
use crate::block_id;
use crate::component::collision_box_component::CollisionBoxComponent;
use crate::component::light_dampening_component::LightDampeningComponent;
use crate::component::mineable_component::MineableComponent;
use crate::component::moveable_component::{MoveableComponent, Movement};
use crate::component::solid_component::SolidComponent;
use crate::component::transparent_component::TransparentComponent;
use crate::const_block;
use crate::state::common::MINECRAFT_CARDINAL_DIRECTION;
use glam::Vec3;

pub const EXPOSED_COPPER_GOLEM_STATUE: BlockDefinition = const_block! {
    identifier: block_id::EXPOSED_COPPER_GOLEM_STATUE,
    states: [MINECRAFT_CARDINAL_DIRECTION],
    components: [
        SolidComponent { solid: false },
        TransparentComponent { transparent: true },
        LightDampeningComponent { dampening: 1 },
        MineableComponent::hardness(1.0),
        MoveableComponent { movement: Movement::Break, sticky: false },
        CollisionBoxComponent::new(Vec3::new(0.25, 0.0, 0.25), Vec3::new(0.5, 0.5, 0.5)),
    ],
    permutations: [],
};
