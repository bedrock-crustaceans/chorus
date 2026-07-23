use crate::block::block_definition::BlockDefinition;
use crate::block::component::light_dampening_component::LightDampeningComponent;
use crate::block::component::light_emission_component::LightEmissionComponent;
use crate::block::component::map_color_component::MapColorComponent;
use crate::block::component::mineable_component::MineableComponent;
use crate::block::component::moveable_component::{MoveableComponent, Movement};
use crate::block::component::transparent_component::TransparentComponent;
use crate::const_block;

pub const DRAGON_EGG: BlockDefinition = const_block! {
    identifier: "minecraft:dragon_egg",
    states: [],
    components: [
        TransparentComponent { transparent: true },
        MapColorComponent { r: 25, g: 25, b: 25, a: 255 },
        LightEmissionComponent { emission: 1 },
        LightDampeningComponent { dampening: 1 },
        MineableComponent::hardness(3.0),
        MoveableComponent { movement: Movement::Break, sticky: false },
    ],
    permutations: [],
};
