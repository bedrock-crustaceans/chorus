use crate::block_definition::BlockDefinition;
use crate::block_id;
use crate::component::light_dampening_component::LightDampeningComponent;
use crate::component::map_color_component::MapColorComponent;
use crate::component::mineable_component::MineableComponent;
use crate::component::moveable_component::{MoveableComponent, Movement};
use crate::component::transparent_component::TransparentComponent;
use crate::const_block;

pub const CHORUS_PLANT: BlockDefinition = const_block! {
    identifier: block_id::CHORUS_PLANT,
    states: [],
    components: [
        TransparentComponent { transparent: true },
        MapColorComponent { r: 153, g: 90, b: 205, a: 255 },
        LightDampeningComponent { dampening: 1 },
        MineableComponent::hardness(0.4),
        MoveableComponent { movement: Movement::Break, sticky: false },
    ],
    permutations: [],
};
