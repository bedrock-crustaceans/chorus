use crate::block_definition::BlockDefinition;
use crate::block_id;
use crate::component::map_color_component::MapColorComponent;
use crate::component::mineable_component::MineableComponent;
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
