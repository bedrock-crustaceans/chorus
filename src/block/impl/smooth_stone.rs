use crate::block::block_definition::BlockDefinition;
use crate::block::block_id;
use crate::block::component::map_color_component::MapColorComponent;
use crate::block::component::mineable_component::MineableComponent;
use crate::const_block;

pub const SMOOTH_STONE: BlockDefinition = const_block! {
    identifier: block_id::SMOOTH_STONE,
    states: [],
    components: [
        MapColorComponent { r: 112, g: 112, b: 112, a: 255 },
        MineableComponent::hardness(1.5),
    ],
    permutations: [],
};
