use crate::block::block_definition::BlockDefinition;
use crate::block::block_id;
use crate::block::component::map_color_component::MapColorComponent;
use crate::block::component::mineable_component::MineableComponent;
use crate::const_block;

pub const INFESTED_MOSSY_STONE_BRICKS: BlockDefinition = const_block! {
    identifier: block_id::INFESTED_MOSSY_STONE_BRICKS,
    states: [],
    components: [
        MapColorComponent { r: 112, g: 112, b: 112, a: 255 },
        MineableComponent::hardness(0.75),
    ],
    permutations: [],
};
