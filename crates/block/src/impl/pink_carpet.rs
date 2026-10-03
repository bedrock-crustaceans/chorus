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
use glam::Vec3;

pub const PINK_CARPET: BlockDefinition = const_block! {
    identifier: block_id::PINK_CARPET,
    states: [],
    components: [
        SolidComponent { solid: false },
        TransparentComponent { transparent: true },
        MapColorComponent { r: 242, g: 127, b: 165, a: 255 },
        LightDampeningComponent { dampening: 1 },
        MineableComponent::hardness(0.1),
        MoveableComponent { movement: Movement::Break, sticky: false },
        CollisionBoxComponent::new(Vec3::new(0.0, 0.0, 0.0), Vec3::new(1.0, 0.0625, 1.0)),
    ],
    permutations: [],
};
