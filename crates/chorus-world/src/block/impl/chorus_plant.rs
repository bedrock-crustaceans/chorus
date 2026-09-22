use crate::block::block_definition::BlockDefinition;
use crate::block::component::light_dampening_component::LightDampeningComponent;
use crate::block::component::map_color_component::MapColorComponent;
use crate::block::component::mineable_component::MineableComponent;
use crate::block::component::moveable_component::{MoveableComponent, Movement};
use crate::block::component::transparent_component::TransparentComponent;
use crate::const_block;

pub const CHORUS_PLANT: BlockDefinition = const_block! {
    identifier: "minecraft:chorus_plant",
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
