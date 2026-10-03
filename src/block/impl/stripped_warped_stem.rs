use crate::block::block_definition::BlockDefinition;
use crate::block::block_id;
use crate::block::component::map_color_component::MapColorComponent;
use crate::block::component::mineable_component::MineableComponent;
use crate::block::state::common::PILLAR_AXIS;
use crate::const_block;

pub const STRIPPED_WARPED_STEM: BlockDefinition = const_block! {
    identifier: block_id::STRIPPED_WARPED_STEM,
    states: [PILLAR_AXIS],
    components: [
        MapColorComponent { r: 58, g: 142, b: 140, a: 255 },
        MineableComponent::hardness(2.0),
    ],
    permutations: [],
};
