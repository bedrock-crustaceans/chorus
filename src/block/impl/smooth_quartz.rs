use crate::block::block_definition::BlockDefinition;
use crate::block::block_id;
use crate::block::component::map_color_component::MapColorComponent;
use crate::block::component::mineable_component::MineableComponent;
use crate::block::state::common::PILLAR_AXIS;
use crate::const_block;

pub const SMOOTH_QUARTZ: BlockDefinition = const_block! {
    identifier: block_id::SMOOTH_QUARTZ,
    states: [PILLAR_AXIS],
    components: [
        MapColorComponent { r: 255, g: 252, b: 245, a: 255 },
        MineableComponent::hardness(0.8),
    ],
    permutations: [],
};
