use crate::block::block_definition::BlockDefinition;
use crate::block::component::flammable_component::FlammableComponent;
use crate::block::component::map_color_component::MapColorComponent;
use crate::block::component::mineable_component::MineableComponent;
use crate::block::state::common::PILLAR_AXIS;
use crate::const_block;

pub const STRIPPED_OAK_LOG: BlockDefinition = const_block! {
    identifier: "minecraft:stripped_oak_log",
    states: [PILLAR_AXIS],
    components: [
        MapColorComponent { r: 143, g: 119, b: 72, a: 255 },
        FlammableComponent { catch_chance: 5, destroy_chance: 10 },
        MineableComponent::hardness(2.0),
    ],
    permutations: [],
};
