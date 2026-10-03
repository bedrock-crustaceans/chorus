use crate::block_definition::BlockDefinition;
use crate::block_id;
use crate::component::collision_box_component::CollisionBoxComponent;
use crate::component::light_dampening_component::LightDampeningComponent;
use crate::component::map_color_component::MapColorComponent;
use crate::component::moveable_component::{MoveableComponent, Movement};
use crate::component::solid_component::SolidComponent;
use crate::component::transparent_component::TransparentComponent;
use crate::const_block;
use glam::Vec3;

pub const HEAVY_CORE: BlockDefinition = const_block! {
    identifier: block_id::HEAVY_CORE,
    states: [],
    components: [
        SolidComponent { solid: false },
        TransparentComponent { transparent: true },
        MapColorComponent { r: 167, g: 167, b: 167, a: 255 },
        LightDampeningComponent { dampening: 1 },
        MoveableComponent { movement: Movement::Both, sticky: false },
        CollisionBoxComponent::new(Vec3::new(0.25, 0.0, 0.25), Vec3::new(0.5, 0.5, 0.5)),
    ],
    permutations: [],
};
