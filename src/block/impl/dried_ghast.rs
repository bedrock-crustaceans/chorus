use crate::block::block_definition::BlockDefinition;
use crate::block::component::light_dampening_component::LightDampeningComponent;
use crate::block::component::map_color_component::MapColorComponent;
use crate::block::component::transparent_component::TransparentComponent;
use crate::block::state::common::{MINECRAFT_CARDINAL_DIRECTION, REHYDRATION_LEVEL};
use crate::const_block;

pub const DRIED_GHAST: BlockDefinition = const_block! {
    identifier: "minecraft:dried_ghast",
    states: [MINECRAFT_CARDINAL_DIRECTION, REHYDRATION_LEVEL],
    components: [
        TransparentComponent { transparent: true },
        MapColorComponent { r: 76, g: 76, b: 76, a: 255 },
        LightDampeningComponent { dampening: 1 },
    ],
    permutations: [],
};
