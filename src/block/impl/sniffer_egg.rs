use crate::block::block_definition::BlockDefinition;
use crate::block::component::light_dampening_component::LightDampeningComponent;
use crate::block::component::map_color_component::MapColorComponent;
use crate::block::component::transparent_component::TransparentComponent;
use crate::block::state::common::CRACKED_STATE;
use crate::const_block;

pub const SNIFFER_EGG: BlockDefinition = const_block! {
    identifier: "minecraft:sniffer_egg",
    states: [CRACKED_STATE],
    components: [
        TransparentComponent { transparent: true },
        MapColorComponent { r: 153, g: 51, b: 51, a: 255 },
        LightDampeningComponent { dampening: 1 },
    ],
    permutations: [],
};
