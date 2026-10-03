use crate::block_definition::BlockDefinition;
use crate::block_id;
use crate::component::map_color_component::MapColorComponent;
use crate::component::mineable_component::MineableComponent;
use crate::const_block;
use crate::state::common::MINECRAFT_VERTICAL_HALF;

pub const TUFF_DOUBLE_SLAB: BlockDefinition = const_block! {
    identifier: block_id::TUFF_DOUBLE_SLAB,
    states: [MINECRAFT_VERTICAL_HALF],
    components: [
        MapColorComponent { r: 57, g: 41, b: 35, a: 255 },
        MineableComponent::hardness(1.5),
    ],
    permutations: [],
};
