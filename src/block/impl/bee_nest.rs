use crate::block::block_definition::BlockDefinition;
use crate::block::component::flammable_component::FlammableComponent;
use crate::block::component::map_color_component::MapColorComponent;
use crate::block::component::mineable_component::MineableComponent;
use crate::block::state::common::{DIRECTION, HONEY_LEVEL};
use crate::const_block;

pub const BEE_NEST: BlockDefinition = const_block! {
    identifier: "minecraft:bee_nest",
    states: [DIRECTION, HONEY_LEVEL],
    components: [
        MapColorComponent { r: 229, g: 229, b: 51, a: 255 },
        FlammableComponent { catch_chance: 30, destroy_chance: 60 },
        MineableComponent::hardness(0.3),
    ],
    permutations: [],
};
