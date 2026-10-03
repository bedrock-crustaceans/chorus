use crate::block_definition::BlockDefinition;
use crate::block_id;
use crate::component::light_dampening_component::LightDampeningComponent;
use crate::component::map_color_component::MapColorComponent;
use crate::component::mineable_component::MineableComponent;
use crate::component::moveable_component::{MoveableComponent, Movement};
use crate::component::solid_component::SolidComponent;
use crate::component::transparent_component::TransparentComponent;
use crate::const_block;

pub const BROWN_SHULKER_BOX: BlockDefinition = const_block! {
    identifier: block_id::BROWN_SHULKER_BOX,
    states: [],
    components: [
        SolidComponent { solid: false },
        TransparentComponent { transparent: true },
        MapColorComponent { r: 102, g: 76, b: 51, a: 255 },
        LightDampeningComponent { dampening: 1 },
        MineableComponent::hardness(2.0),
        MoveableComponent { movement: Movement::Break, sticky: false },
    ],
    permutations: [],
};
