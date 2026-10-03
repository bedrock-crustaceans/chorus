use crate::block_definition::BlockDefinition;
use crate::block_id;
use crate::component::flammable_component::FlammableComponent;
use crate::component::map_color_component::MapColorComponent;
use crate::component::mineable_component::MineableComponent;
use crate::const_block;
use crate::state::common::{DIRECTION, HONEY_LEVEL};

pub const BEE_NEST: BlockDefinition = const_block! {
    identifier: block_id::BEE_NEST,
    states: [DIRECTION, HONEY_LEVEL],
    components: [
        MapColorComponent { r: 229, g: 229, b: 51, a: 255 },
        FlammableComponent { catch_chance: 30, destroy_chance: 60 },
        MineableComponent::hardness(0.3),
    ],
    permutations: [],
};
