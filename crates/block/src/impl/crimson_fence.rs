use crate::block_definition::BlockDefinition;
use crate::block_id;
use crate::component::collision_box_component::CollisionBoxComponent;
use crate::component::light_dampening_component::LightDampeningComponent;
use crate::component::map_color_component::MapColorComponent;
use crate::component::mineable_component::MineableComponent;
use crate::component::transparent_component::TransparentComponent;
use crate::const_block;
use crate::state::common::{MINECRAFT_CONNECTION_EAST, MINECRAFT_CONNECTION_NORTH, MINECRAFT_CONNECTION_SOUTH, MINECRAFT_CONNECTION_WEST};
use glam::Vec3;

pub const CRIMSON_FENCE: BlockDefinition = const_block! {
    identifier: block_id::CRIMSON_FENCE,
    states: [MINECRAFT_CONNECTION_EAST, MINECRAFT_CONNECTION_NORTH, MINECRAFT_CONNECTION_SOUTH, MINECRAFT_CONNECTION_WEST],
    components: [
        TransparentComponent { transparent: true },
        MapColorComponent { r: 112, g: 2, b: 0, a: 255 },
        LightDampeningComponent { dampening: 1 },
        MineableComponent::hardness(2.0),
        CollisionBoxComponent::new(Vec3::new(0.0, 0.0, 0.0), Vec3::new(1.0, 1.5, 1.0)),
    ],
    permutations: [],
};
