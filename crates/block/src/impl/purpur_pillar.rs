use crate::block_definition::BlockDefinition;
use crate::block_id;
use crate::component::map_color_component::MapColorComponent;
use crate::component::mineable_component::MineableComponent;
use crate::const_block;
use crate::state::common::PILLAR_AXIS;

pub const PURPUR_PILLAR: BlockDefinition = const_block! {
    identifier: block_id::PURPUR_PILLAR,
    states: [PILLAR_AXIS],
    components: [
        MapColorComponent { r: 178, g: 76, b: 216, a: 255 },
        MineableComponent::hardness(1.5),
    ],
    permutations: [],
};
