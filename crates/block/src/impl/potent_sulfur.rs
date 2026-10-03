use crate::block_definition::BlockDefinition;
use crate::block_id;
use crate::component::map_color_component::MapColorComponent;
use crate::component::mineable_component::MineableComponent;
use crate::const_block;
use crate::state::common::POTENT_SULFUR_STATE;

pub const POTENT_SULFUR: BlockDefinition = const_block! {
    identifier: block_id::POTENT_SULFUR,
    states: [POTENT_SULFUR_STATE],
    components: [
        MapColorComponent { r: 250, g: 238, b: 77, a: 255 },
        MineableComponent::hardness(1.5),
    ],
    permutations: [],
};
