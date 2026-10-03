use crate::block::block_definition::BlockDefinition;
use crate::block::block_id;
use crate::block::component::map_color_component::MapColorComponent;
use crate::block::component::mineable_component::MineableComponent;
use crate::const_block;

pub const WARPED_WART_BLOCK: BlockDefinition = const_block! {
    identifier: block_id::WARPED_WART_BLOCK,
    states: [],
    components: [
        MapColorComponent { r: 20, g: 180, b: 133, a: 255 },
        MineableComponent::hardness(1.0),
    ],
    permutations: [],
};
