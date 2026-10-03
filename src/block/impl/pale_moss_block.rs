use crate::block::block_definition::BlockDefinition;
use crate::block::block_id;
use crate::block::component::map_color_component::MapColorComponent;
use crate::block::component::mineable_component::MineableComponent;
use crate::const_block;

pub const PALE_MOSS_BLOCK: BlockDefinition = const_block! {
    identifier: block_id::PALE_MOSS_BLOCK,
    states: [],
    components: [
        MapColorComponent { r: 153, g: 153, b: 153, a: 255 },
        MineableComponent::hardness(0.1),
    ],
    permutations: [],
};
