use crate::block::block_definition::BlockDefinition;
use crate::block::component::light_dampening_component::LightDampeningComponent;
use crate::block::component::map_color_component::MapColorComponent;
use crate::block::component::mineable_component::MineableComponent;
use crate::block::component::transparent_component::TransparentComponent;
use crate::block::state::common::{FACING_DIRECTION, TOGGLE_BIT};
use crate::const_block;

pub const HOPPER: BlockDefinition = const_block! {
    identifier: "minecraft:hopper",
    states: [FACING_DIRECTION, TOGGLE_BIT],
    components: [
        TransparentComponent { transparent: true },
        MapColorComponent { r: 112, g: 112, b: 112, a: 255 },
        LightDampeningComponent { dampening: 1 },
        MineableComponent::hardness(3.0),
    ],
    permutations: [],
};
