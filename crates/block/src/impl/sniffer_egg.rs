use crate::block_definition::BlockDefinition;
use crate::block_id;
use crate::component::light_dampening_component::LightDampeningComponent;
use crate::component::map_color_component::MapColorComponent;
use crate::component::transparent_component::TransparentComponent;
use crate::const_block;
use crate::state::common::CRACKED_STATE;

pub const SNIFFER_EGG: BlockDefinition = const_block! {
    identifier: block_id::SNIFFER_EGG,
    states: [CRACKED_STATE],
    components: [
        TransparentComponent { transparent: true },
        MapColorComponent { r: 153, g: 51, b: 51, a: 255 },
        LightDampeningComponent { dampening: 1 },
    ],
    permutations: [],
};
