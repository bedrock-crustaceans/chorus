use crate::block_definition::BlockDefinition;
use crate::block_id;
use crate::component::collision_box_component::CollisionBoxComponent;
use crate::component::light_dampening_component::LightDampeningComponent;
use crate::component::map_color_component::MapColorComponent;
use crate::component::mineable_component::MineableComponent;
use crate::component::moveable_component::{MoveableComponent, Movement};
use crate::component::solid_component::SolidComponent;
use crate::component::transparent_component::TransparentComponent;
use crate::const_block;
use crate::state::common::{WALL_CONNECTION_TYPE_EAST, WALL_CONNECTION_TYPE_NORTH, WALL_CONNECTION_TYPE_SOUTH, WALL_CONNECTION_TYPE_WEST, WALL_POST_BIT};
use glam::Vec3;

pub const BORDER_BLOCK: BlockDefinition = const_block! {
    identifier: block_id::BORDER_BLOCK,
    states: [WALL_CONNECTION_TYPE_EAST, WALL_CONNECTION_TYPE_NORTH, WALL_CONNECTION_TYPE_SOUTH, WALL_CONNECTION_TYPE_WEST, WALL_POST_BIT],
    components: [
        SolidComponent { solid: false },
        TransparentComponent { transparent: true },
        MapColorComponent { r: 255, g: 0, b: 0, a: 255 },
        LightDampeningComponent { dampening: 1 },
        MineableComponent::hardness(-1.0),
        MoveableComponent { movement: Movement::None, sticky: false },
        CollisionBoxComponent::new(Vec3::new(0.0, 4.9E-324, 0.0), Vec3::new(1.0, f32::MAX, 1.0)),
    ],
    permutations: [],
};
