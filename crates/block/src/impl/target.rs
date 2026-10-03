use crate::block_definition::BlockDefinition;
use crate::block_id;
use crate::component::flammable_component::FlammableComponent;
use crate::component::light_dampening_component::LightDampeningComponent;
use crate::component::map_color_component::MapColorComponent;
use crate::component::mineable_component::MineableComponent;
use crate::component::transparent_component::TransparentComponent;
use crate::const_block;

pub const TARGET: BlockDefinition = const_block! {
    identifier: block_id::TARGET,
    states: [],
    components: [
        TransparentComponent { transparent: true },
        MapColorComponent { r: 255, g: 255, b: 255, a: 255 },
        LightDampeningComponent { dampening: 1 },
        FlammableComponent { catch_chance: 0, destroy_chance: 15 },
        MineableComponent::hardness(0.5),
    ],
    permutations: [],
};
