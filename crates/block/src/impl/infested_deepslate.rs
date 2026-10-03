use crate::block_definition::BlockDefinition;
use crate::block_id;
use crate::component::map_color_component::MapColorComponent;
use crate::component::mineable_component::MineableComponent;
use crate::const_block;
use crate::state::common::PILLAR_AXIS;

pub const INFESTED_DEEPSLATE: BlockDefinition = const_block! {
    identifier: block_id::INFESTED_DEEPSLATE,
    states: [PILLAR_AXIS],
    components: [
        MapColorComponent { r: 100, g: 100, b: 100, a: 255 },
        MineableComponent::hardness(1.5),
    ],
    permutations: [],
};
