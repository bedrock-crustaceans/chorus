use crate::block::block_definition::BlockDefinition;
use crate::block::block_id;
use crate::block::component::map_color_component::MapColorComponent;
use crate::block::component::mineable_component::MineableComponent;
use crate::const_block;

pub const NOTEBLOCK: BlockDefinition = const_block! {
    identifier: block_id::NOTEBLOCK,
    states: [],
    components: [
        MapColorComponent { r: 143, g: 119, b: 72, a: 255 },
        MineableComponent::hardness(0.8),
    ],
    permutations: [],
};
