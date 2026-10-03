use crate::block_definition::BlockDefinition;
use crate::block_id;
use crate::component::map_color_component::MapColorComponent;
use crate::component::mineable_component::MineableComponent;
use crate::const_block;
use crate::state::common::DIRECTION;

pub const LAB_TABLE: BlockDefinition = const_block! {
    identifier: block_id::LAB_TABLE,
    states: [DIRECTION],
    components: [
        MapColorComponent { r: 143, g: 119, b: 72, a: 255 },
        MineableComponent::hardness(2.5),
    ],
    permutations: [],
};
