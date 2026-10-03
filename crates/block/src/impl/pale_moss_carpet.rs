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
use crate::state::common::{PALE_MOSS_CARPET_SIDE_EAST, PALE_MOSS_CARPET_SIDE_NORTH, PALE_MOSS_CARPET_SIDE_SOUTH, PALE_MOSS_CARPET_SIDE_WEST, UPPER_BLOCK_BIT};
use glam::Vec3;

pub const PALE_MOSS_CARPET: BlockDefinition = const_block! {
    identifier: block_id::PALE_MOSS_CARPET,
    states: [PALE_MOSS_CARPET_SIDE_EAST, PALE_MOSS_CARPET_SIDE_NORTH, PALE_MOSS_CARPET_SIDE_SOUTH, PALE_MOSS_CARPET_SIDE_WEST, UPPER_BLOCK_BIT],
    components: [
        SolidComponent { solid: false },
        TransparentComponent { transparent: true },
        MapColorComponent { r: 153, g: 153, b: 153, a: 255 },
        LightDampeningComponent { dampening: 1 },
        MineableComponent::hardness(0.1),
        MoveableComponent { movement: Movement::Break, sticky: false },
        CollisionBoxComponent::new(Vec3::new(0.0, 0.0, 0.0), Vec3::new(1.0, 0.0625, 1.0)),
    ],
    permutations: [],
};
