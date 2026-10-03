use crate::block_definition::BlockDefinition;
use crate::block_id;
use crate::component::map_color_component::MapColorComponent;
use crate::component::mineable_component::MineableComponent;
use crate::const_block;

pub const CINNABAR_BRICKS: BlockDefinition = const_block! {
    identifier: block_id::CINNABAR_BRICKS,
    states: [],
    components: [
        MapColorComponent { r: 153, g: 51, b: 51, a: 255 },
        MineableComponent::hardness(1.5),
    ],
    permutations: [],
};
