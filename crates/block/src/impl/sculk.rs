use crate::block_definition::BlockDefinition;
use crate::block_id;
use crate::component::map_color_component::MapColorComponent;
use crate::component::mineable_component::MineableComponent;
use crate::const_block;

pub const SCULK: BlockDefinition = const_block! {
    identifier: block_id::SCULK,
    states: [],
    components: [
        MapColorComponent { r: 13, g: 18, b: 23, a: 255 },
        MineableComponent::hardness(0.2),
    ],
    permutations: [],
};
