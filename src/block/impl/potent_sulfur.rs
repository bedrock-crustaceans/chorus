use crate::block::block_definition::BlockDefinition;
use crate::block::block_id;
use crate::block::component::map_color_component::MapColorComponent;
use crate::block::component::mineable_component::MineableComponent;
use crate::block::state::common::POTENT_SULFUR_STATE;
use crate::const_block;

pub const POTENT_SULFUR: BlockDefinition = const_block! {
    identifier: block_id::POTENT_SULFUR,
    states: [POTENT_SULFUR_STATE],
    components: [
        MapColorComponent { r: 250, g: 238, b: 77, a: 255 },
        MineableComponent::hardness(1.5),
    ],
    permutations: [],
};
