use crate::block_definition::BlockDefinition;
use crate::block_id;
use crate::component::map_color_component::MapColorComponent;
use crate::component::mineable_component::MineableComponent;
use crate::const_block;

pub const IRON_BLOCK: BlockDefinition = const_block! {
    identifier: block_id::IRON_BLOCK,
    states: [],
    components: [
        MapColorComponent { r: 167, g: 167, b: 167, a: 255 },
        MineableComponent::hardness(5.0),
    ],
    permutations: [],
};
