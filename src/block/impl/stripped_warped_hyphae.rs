use crate::block::block_definition::BlockDefinition;
use crate::block::component::flammable_component::FlammableComponent;
use crate::block::component::map_color_component::MapColorComponent;
use crate::block::component::mineable_component::MineableComponent;
use crate::block::state::common::PILLAR_AXIS;
use crate::const_block;

pub const STRIPPED_WARPED_HYPHAE: BlockDefinition = const_block! {
    identifier: "minecraft:stripped_warped_hyphae",
    states: [PILLAR_AXIS],
    components: [
        MapColorComponent { r: 86, g: 44, b: 62, a: 255 },
        FlammableComponent { catch_chance: 5, destroy_chance: 10 },
        MineableComponent::hardness(2.0),
    ],
    permutations: [],
};
