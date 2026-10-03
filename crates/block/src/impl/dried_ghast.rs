use crate::block_definition::BlockDefinition;
use crate::block_id;
use crate::component::light_dampening_component::LightDampeningComponent;
use crate::component::map_color_component::MapColorComponent;
use crate::component::transparent_component::TransparentComponent;
use crate::const_block;
use crate::state::common::{MINECRAFT_CARDINAL_DIRECTION, REHYDRATION_LEVEL};

pub const DRIED_GHAST: BlockDefinition = const_block! {
    identifier: block_id::DRIED_GHAST,
    states: [MINECRAFT_CARDINAL_DIRECTION, REHYDRATION_LEVEL],
    components: [
        TransparentComponent { transparent: true },
        MapColorComponent { r: 76, g: 76, b: 76, a: 255 },
        LightDampeningComponent { dampening: 1 },
    ],
    permutations: [],
};
