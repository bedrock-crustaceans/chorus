use crate::block_definition::BlockDefinition;
use crate::block_id;
use crate::component::flammable_component::FlammableComponent;
use crate::component::friction_component::FrictionComponent;
use crate::component::light_dampening_component::LightDampeningComponent;
use crate::component::map_color_component::MapColorComponent;
use crate::component::mineable_component::MineableComponent;
use crate::component::transparent_component::TransparentComponent;
use crate::const_block;

pub const ICE: BlockDefinition = const_block! {
    identifier: block_id::ICE,
    states: [],
    components: [
        TransparentComponent { transparent: true },
        MapColorComponent { r: 160, g: 160, b: 255, a: 255 },
        FrictionComponent { friction: 0.98 },
        LightDampeningComponent { dampening: 2 },
        FlammableComponent { catch_chance: -1, destroy_chance: 0 },
        MineableComponent::hardness(0.5),
    ],
    permutations: [],
};
