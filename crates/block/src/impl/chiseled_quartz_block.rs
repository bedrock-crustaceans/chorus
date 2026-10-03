use crate::block_definition::BlockDefinition;
use crate::block_id;
use crate::component::map_color_component::MapColorComponent;
use crate::component::mineable_component::MineableComponent;
use crate::const_block;
use crate::state::common::PILLAR_AXIS;

pub const CHISELED_QUARTZ_BLOCK: BlockDefinition = const_block! {
    identifier: block_id::CHISELED_QUARTZ_BLOCK,
    states: [PILLAR_AXIS],
    components: [
        MapColorComponent { r: 255, g: 252, b: 245, a: 255 },
        MineableComponent::hardness(0.8),
    ],
    permutations: [],
};
