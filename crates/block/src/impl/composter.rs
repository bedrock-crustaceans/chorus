use crate::block_definition::BlockDefinition;
use crate::block_id;
use crate::component::map_color_component::MapColorComponent;
use crate::component::mineable_component::MineableComponent;
use crate::const_block;
use crate::state::common::COMPOSTER_FILL_LEVEL;

pub const COMPOSTER: BlockDefinition = const_block! {
    identifier: block_id::COMPOSTER,
    states: [COMPOSTER_FILL_LEVEL],
    components: [
        MapColorComponent { r: 143, g: 119, b: 72, a: 255 },
        MineableComponent::hardness(0.6),
    ],
    permutations: [],
};
