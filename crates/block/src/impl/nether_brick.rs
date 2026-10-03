use crate::block_definition::BlockDefinition;
use crate::block_id;
use crate::component::map_color_component::MapColorComponent;
use crate::component::mineable_component::MineableComponent;
use crate::const_block;

pub const NETHER_BRICK: BlockDefinition = const_block! {
    identifier: block_id::NETHER_BRICK,
    states: [],
    components: [
        MapColorComponent { r: 112, g: 2, b: 0, a: 255 },
        MineableComponent::hardness(2.0),
    ],
    permutations: [],
};
