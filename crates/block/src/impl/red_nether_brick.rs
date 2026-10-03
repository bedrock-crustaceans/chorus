use crate::block_definition::BlockDefinition;
use crate::block_id;
use crate::component::map_color_component::MapColorComponent;
use crate::component::mineable_component::MineableComponent;
use crate::const_block;

pub const RED_NETHER_BRICK: BlockDefinition = const_block! {
    identifier: block_id::RED_NETHER_BRICK,
    states: [],
    components: [
        MapColorComponent { r: 112, g: 2, b: 0, a: 255 },
        MineableComponent::hardness(2.0),
    ],
    permutations: [],
};
