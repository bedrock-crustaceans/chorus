use crate::block_definition::BlockDefinition;
use crate::block_id;
use crate::component::light_dampening_component::LightDampeningComponent;
use crate::component::map_color_component::MapColorComponent;
use crate::component::mineable_component::MineableComponent;
use crate::component::moveable_component::{MoveableComponent, Movement};
use crate::component::transparent_component::TransparentComponent;
use crate::const_block;

pub const SLIME: BlockDefinition = const_block! {
    identifier: block_id::SLIME,
    states: [],
    components: [
        TransparentComponent { transparent: true },
        MapColorComponent { r: 127, g: 178, b: 56, a: 255 },
        LightDampeningComponent { dampening: 1 },
        MineableComponent::hardness(0.0),
        MoveableComponent { movement: Movement::Both, sticky: true },
    ],
    permutations: [],
};
