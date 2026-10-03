use crate::block_definition::BlockDefinition;
use crate::block_id;
use crate::component::collision_box_component::CollisionBoxComponent;
use crate::component::internal_friction_component::InternalFrictionComponent;
use crate::component::light_dampening_component::LightDampeningComponent;
use crate::component::map_color_component::MapColorComponent;
use crate::component::mineable_component::MineableComponent;
use crate::component::moveable_component::{MoveableComponent, Movement};
use crate::component::solid_component::SolidComponent;
use crate::component::transparent_component::TransparentComponent;
use crate::const_block;
use crate::state::common::REDSTONE_SIGNAL;
use glam::Vec3;

pub const STONE_PRESSURE_PLATE: BlockDefinition = const_block! {
    identifier: block_id::STONE_PRESSURE_PLATE,
    states: [REDSTONE_SIGNAL],
    components: [
        SolidComponent { solid: false },
        TransparentComponent { transparent: true },
        MapColorComponent { r: 112, g: 112, b: 112, a: 255 },
        InternalFrictionComponent { internal_friction: 0.95 },
        LightDampeningComponent { dampening: 1 },
        MineableComponent::hardness(0.5),
        MoveableComponent { movement: Movement::Break, sticky: false },
        CollisionBoxComponent { origin: Vec3::new(0.0625, 0.0, 0.0625), size: Vec3::new(0.875, 0.25, 0.875), enabled: false },
    ],
    permutations: [],
};
