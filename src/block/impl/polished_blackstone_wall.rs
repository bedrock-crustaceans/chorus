use crate::block::block_definition::BlockDefinition;
use crate::block::component::collision_box_component::CollisionBoxComponent;
use crate::block::component::light_dampening_component::LightDampeningComponent;
use crate::block::component::map_color_component::MapColorComponent;
use crate::block::component::mineable_component::MineableComponent;
use crate::block::component::solid_component::SolidComponent;
use crate::block::component::transparent_component::TransparentComponent;
use crate::block::state::common::{WALL_CONNECTION_TYPE_EAST, WALL_CONNECTION_TYPE_NORTH, WALL_CONNECTION_TYPE_SOUTH, WALL_CONNECTION_TYPE_WEST, WALL_POST_BIT};
use crate::const_block;
use vek::Vec3;

pub const POLISHED_BLACKSTONE_WALL: BlockDefinition = const_block! {
    identifier: "minecraft:polished_blackstone_wall",
    states: [WALL_CONNECTION_TYPE_EAST, WALL_CONNECTION_TYPE_NORTH, WALL_CONNECTION_TYPE_SOUTH, WALL_CONNECTION_TYPE_WEST, WALL_POST_BIT],
    components: [
        SolidComponent { solid: false },
        TransparentComponent { transparent: true },
        MapColorComponent { r: 25, g: 25, b: 25, a: 255 },
        LightDampeningComponent { dampening: 1 },
        MineableComponent::hardness(1.5),
        CollisionBoxComponent::new(Vec3::new(0.0, 0.0, 0.0), Vec3::new(1.0, 1.5, 1.0)),
    ],
    permutations: [],
};
