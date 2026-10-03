use crate::block_definition::BlockDefinition;
use crate::block_id;
use crate::component::map_color_component::MapColorComponent;
use crate::component::mineable_component::MineableComponent;
use crate::const_block;

pub const CHISELED_RESIN_BRICKS: BlockDefinition = const_block! {
    identifier: block_id::CHISELED_RESIN_BRICKS,
    states: [],
    components: [
        MapColorComponent { r: 159, g: 82, b: 36, a: 255 },
        MineableComponent::hardness(1.5),
    ],
    permutations: [],
};
