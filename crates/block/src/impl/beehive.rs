use crate::block_definition::BlockDefinition;
use crate::block_id;
use crate::component::flammable_component::FlammableComponent;
use crate::component::map_color_component::MapColorComponent;
use crate::component::mineable_component::MineableComponent;
use crate::const_block;
use crate::state::common::{DIRECTION, HONEY_LEVEL};

pub const BEEHIVE: BlockDefinition = const_block! {
    identifier: block_id::BEEHIVE,
    states: [DIRECTION, HONEY_LEVEL],
    components: [
        MapColorComponent { r: 143, g: 119, b: 72, a: 255 },
        FlammableComponent { catch_chance: 5, destroy_chance: 20 },
        MineableComponent::hardness(0.6),
    ],
    permutations: [],
};
