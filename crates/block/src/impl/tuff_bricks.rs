use crate::block_definition::BlockDefinition;
use crate::block_id;
use crate::component::map_color_component::MapColorComponent;
use crate::component::mineable_component::MineableComponent;
use crate::const_block;

pub const TUFF_BRICKS: BlockDefinition = const_block! {
    identifier: block_id::TUFF_BRICKS,
    states: [],
    components: [
        MapColorComponent { r: 57, g: 41, b: 35, a: 255 },
        MineableComponent::hardness(1.5),
    ],
    permutations: [],
};
