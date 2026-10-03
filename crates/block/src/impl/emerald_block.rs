use crate::block_definition::BlockDefinition;
use crate::block_id;
use crate::component::map_color_component::MapColorComponent;
use crate::component::mineable_component::MineableComponent;
use crate::const_block;

pub const EMERALD_BLOCK: BlockDefinition = const_block! {
    identifier: block_id::EMERALD_BLOCK,
    states: [],
    components: [
        MapColorComponent { r: 0, g: 217, b: 58, a: 255 },
        MineableComponent::hardness(5.0),
    ],
    permutations: [],
};
