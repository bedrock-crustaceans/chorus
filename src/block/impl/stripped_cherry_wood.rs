use crate::block::block_definition::BlockDefinition;
use crate::block::component::flammable_component::FlammableComponent;
use crate::block::component::map_color_component::MapColorComponent;
use crate::block::component::mineable_component::MineableComponent;
use crate::block::state::common::PILLAR_AXIS;
use crate::const_block;

pub const STRIPPED_CHERRY_WOOD: BlockDefinition = const_block! {
    identifier: "minecraft:stripped_cherry_wood",
    states: [PILLAR_AXIS],
    components: [
        MapColorComponent { r: 160, g: 77, b: 78, a: 255 },
        FlammableComponent { catch_chance: 5, destroy_chance: 5 },
        MineableComponent::hardness(2.0),
    ],
    permutations: [],
};
