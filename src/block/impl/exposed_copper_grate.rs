use crate::block::block_definition::BlockDefinition;
use crate::block::component::collision_box_component::CollisionBoxComponent;
use crate::block::component::light_dampening_component::LightDampeningComponent;
use crate::block::component::map_color_component::MapColorComponent;
use crate::block::component::mineable_component::MineableComponent;
use crate::block::component::moveable_component::{MoveableComponent, Movement};
use crate::block::component::solid_component::SolidComponent;
use crate::block::component::transparent_component::TransparentComponent;
use crate::const_block;

pub const EXPOSED_COPPER_GRATE: BlockDefinition = const_block! {
    identifier: "minecraft:exposed_copper_grate",
    states: [],
    components: [
        SolidComponent { solid: false },
        TransparentComponent { transparent: true },
        MapColorComponent { r: 135, g: 107, b: 98, a: 255 },
        LightDampeningComponent { dampening: 1 },
        MineableComponent::hardness(3.0),
        MoveableComponent { movement: Movement::Both, sticky: false },
        CollisionBoxComponent::enabled(false),
    ],
    permutations: [],
};
