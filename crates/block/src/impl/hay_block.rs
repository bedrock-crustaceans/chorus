use crate::block_definition::BlockDefinition;
use crate::block_id;
use crate::component::flammable_component::FlammableComponent;
use crate::component::map_color_component::MapColorComponent;
use crate::component::mineable_component::MineableComponent;
use crate::const_block;
use crate::state::common::{DEPRECATED, PILLAR_AXIS};

pub const HAY_BLOCK: BlockDefinition = const_block! {
    identifier: block_id::HAY_BLOCK,
    states: [DEPRECATED, PILLAR_AXIS],
    components: [
        MapColorComponent { r: 229, g: 229, b: 51, a: 255 },
        FlammableComponent { catch_chance: 60, destroy_chance: 20 },
        MineableComponent::hardness(0.5),
    ],
    permutations: [],
};
