use crate::block::block_definition::BlockDefinition;
use crate::block::component::collision_box_component::CollisionBoxComponent;
use crate::block::component::light_dampening_component::LightDampeningComponent;
use crate::block::component::map_color_component::MapColorComponent;
use crate::block::component::moveable_component::{MoveableComponent, Movement};
use crate::block::component::solid_component::SolidComponent;
use crate::block::component::transparent_component::TransparentComponent;
use crate::const_block;
use vek::Vec3;

pub const HEAVY_CORE: BlockDefinition = const_block! {
    identifier: "minecraft:heavy_core",
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
