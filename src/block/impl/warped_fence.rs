use crate::block::block_definition::BlockDefinition;
use crate::block::block_id;
use crate::block::component::collision_box_component::CollisionBoxComponent;
use crate::block::component::light_dampening_component::LightDampeningComponent;
use crate::block::component::map_color_component::MapColorComponent;
use crate::block::component::mineable_component::MineableComponent;
use crate::block::component::transparent_component::TransparentComponent;
use crate::block::state::common::{MINECRAFT_CONNECTION_EAST, MINECRAFT_CONNECTION_NORTH, MINECRAFT_CONNECTION_SOUTH, MINECRAFT_CONNECTION_WEST};
use crate::const_block;
use glam::Vec3;

pub const WARPED_FENCE: BlockDefinition = const_block! {
    identifier: block_id::WARPED_FENCE,
    states: [MINECRAFT_CONNECTION_EAST, MINECRAFT_CONNECTION_NORTH, MINECRAFT_CONNECTION_SOUTH, MINECRAFT_CONNECTION_WEST],
    components: [
        TransparentComponent { transparent: true },
        MapColorComponent { r: 76, g: 127, b: 153, a: 255 },
        LightDampeningComponent { dampening: 1 },
        MineableComponent::hardness(2.0),
        CollisionBoxComponent::new(Vec3::new(0.0, 0.0, 0.0), Vec3::new(1.0, 1.5, 1.0)),
    ],
    permutations: [],
};
