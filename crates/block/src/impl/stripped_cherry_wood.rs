use crate::block_definition::BlockDefinition;
use crate::block_id;
use crate::component::flammable_component::FlammableComponent;
use crate::component::map_color_component::MapColorComponent;
use crate::component::mineable_component::MineableComponent;
use crate::const_block;
use crate::state::common::PILLAR_AXIS;

pub const STRIPPED_CHERRY_WOOD: BlockDefinition = const_block! {
    identifier: block_id::STRIPPED_CHERRY_WOOD,
    states: [PILLAR_AXIS],
    components: [
        MapColorComponent { r: 160, g: 77, b: 78, a: 255 },
        FlammableComponent { catch_chance: 5, destroy_chance: 5 },
        MineableComponent::hardness(2.0),
    ],
    permutations: [],
};
