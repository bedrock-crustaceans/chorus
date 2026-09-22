use crate::block::block_definition::BlockDefinition;
use crate::block::component::flammable_component::FlammableComponent;
use crate::block::component::map_color_component::MapColorComponent;
use crate::block::component::mineable_component::MineableComponent;
use crate::block::state::common::{DIRECTION, HONEY_LEVEL};
use crate::const_block;

pub const BEEHIVE: BlockDefinition = const_block! {
    identifier: "minecraft:beehive",
    states: [DIRECTION, HONEY_LEVEL],
    components: [
        MapColorComponent { r: 143, g: 119, b: 72, a: 255 },
        FlammableComponent { catch_chance: 5, destroy_chance: 20 },
        MineableComponent::hardness(0.6),
    ],
    permutations: [],
};
