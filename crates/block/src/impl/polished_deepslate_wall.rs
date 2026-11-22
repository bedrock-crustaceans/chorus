use crate::block_definition::BlockDefinition;
use crate::block_id;
use crate::component::collision_box_component::CollisionBoxComponent;
use crate::component::light_dampening_component::LightDampeningComponent;
use crate::component::liquid_detection_component::LiquidDetectionComponent;
use crate::component::map_color_component::MapColorComponent;
use crate::component::mineable_component::MineableComponent;
use crate::component::transparent_component::TransparentComponent;
use crate::const_block;
use crate::state::common::{WALL_CONNECTION_TYPE_EAST, WALL_CONNECTION_TYPE_NORTH, WALL_CONNECTION_TYPE_SOUTH, WALL_CONNECTION_TYPE_WEST, WALL_POST_BIT};
use glam::Vec3;

pub static POLISHED_DEEPSLATE_WALL: BlockDefinition = const_block! {
    identifier: block_id::POLISHED_DEEPSLATE_WALL,
    states: [WALL_CONNECTION_TYPE_EAST, WALL_CONNECTION_TYPE_NORTH, WALL_CONNECTION_TYPE_SOUTH, WALL_CONNECTION_TYPE_WEST, WALL_POST_BIT],
    components: [
        TransparentComponent { transparent: true },
        MapColorComponent { r: 100, g: 100, b: 100, a: 255 },
        LightDampeningComponent { dampening: 1 },
        MineableComponent::hardness(3.5),
        CollisionBoxComponent::new(Vec3::new(0.0, 0.0, 0.0), Vec3::new(1.0, 1.5, 1.0)),
        LiquidDetectionComponent::CONTAINS_WATER,
    ],
    permutations: [],
};
