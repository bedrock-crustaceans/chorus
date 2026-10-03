use crate::block_definition::BlockDefinition;
use crate::block_id;
use crate::component::map_color_component::MapColorComponent;
use crate::component::mineable_component::MineableComponent;
use crate::const_block;

pub const QUARTZ_ORE: BlockDefinition = const_block! {
    identifier: block_id::QUARTZ_ORE,
    states: [],
    components: [
        MapColorComponent { r: 112, g: 2, b: 0, a: 255 },
        MineableComponent::hardness(3.0),
    ],
    permutations: [],
};
