use crate::block_definition::BlockDefinition;
use crate::block_id;
use crate::component::map_color_component::MapColorComponent;
use crate::component::mineable_component::MineableComponent;
use crate::const_block;

pub const POLISHED_GRANITE: BlockDefinition = const_block! {
    identifier: block_id::POLISHED_GRANITE,
    states: [],
    components: [
        MapColorComponent { r: 151, g: 109, b: 77, a: 255 },
        MineableComponent::hardness(1.5),
    ],
    permutations: [],
};
