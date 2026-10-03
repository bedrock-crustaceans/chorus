use crate::block_definition::BlockDefinition;
use crate::block_id;
use crate::component::flammable_component::FlammableComponent;
use crate::component::map_color_component::MapColorComponent;
use crate::component::mineable_component::MineableComponent;
use crate::const_block;
use crate::state::common::PILLAR_AXIS;

pub const STRIPPED_ACACIA_WOOD: BlockDefinition = const_block! {
    identifier: block_id::STRIPPED_ACACIA_WOOD,
    states: [PILLAR_AXIS],
    components: [
        MapColorComponent { r: 76, g: 76, b: 76, a: 255 },
        FlammableComponent { catch_chance: 5, destroy_chance: 10 },
        MineableComponent::hardness(2.0),
    ],
    permutations: [],
};
