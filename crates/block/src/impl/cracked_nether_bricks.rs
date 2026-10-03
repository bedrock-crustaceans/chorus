use crate::block_definition::BlockDefinition;
use crate::block_id;
use crate::component::map_color_component::MapColorComponent;
use crate::component::mineable_component::MineableComponent;
use crate::const_block;

pub const CRACKED_NETHER_BRICKS: BlockDefinition = const_block! {
    identifier: block_id::CRACKED_NETHER_BRICKS,
    states: [],
    components: [
        MapColorComponent { r: 112, g: 2, b: 0, a: 255 },
        MineableComponent::hardness(2.0),
    ],
    permutations: [],
};
