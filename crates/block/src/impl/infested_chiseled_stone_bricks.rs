use crate::block_definition::BlockDefinition;
use crate::block_id;
use crate::component::map_color_component::MapColorComponent;
use crate::component::mineable_component::MineableComponent;
use crate::const_block;

pub const INFESTED_CHISELED_STONE_BRICKS: BlockDefinition = const_block! {
    identifier: block_id::INFESTED_CHISELED_STONE_BRICKS,
    states: [],
    components: [
        MapColorComponent { r: 112, g: 112, b: 112, a: 255 },
        MineableComponent::hardness(0.75),
    ],
    permutations: [],
};
