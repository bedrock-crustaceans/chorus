use crate::block_definition::BlockDefinition;
use crate::block_id;
use crate::component::map_color_component::MapColorComponent;
use crate::component::mineable_component::MineableComponent;
use crate::const_block;
use crate::state::common::PILLAR_AXIS;

pub const STRIPPED_WARPED_STEM: BlockDefinition = const_block! {
    identifier: block_id::STRIPPED_WARPED_STEM,
    states: [PILLAR_AXIS],
    components: [
        MapColorComponent { r: 58, g: 142, b: 140, a: 255 },
        MineableComponent::hardness(2.0),
    ],
    permutations: [],
};
