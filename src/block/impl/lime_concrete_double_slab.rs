use crate::block::block_definition::BlockDefinition;
use crate::block::block_id;
use crate::block::component::map_color_component::MapColorComponent;
use crate::block::component::mineable_component::MineableComponent;
use crate::block::state::common::MINECRAFT_VERTICAL_HALF;
use crate::const_block;

pub const LIME_CONCRETE_DOUBLE_SLAB: BlockDefinition = const_block! {
    identifier: block_id::LIME_CONCRETE_DOUBLE_SLAB,
    states: [MINECRAFT_VERTICAL_HALF],
    components: [
        MapColorComponent { r: 127, g: 204, b: 25, a: 255 },
        MineableComponent::hardness(1.8),
    ],
    permutations: [],
};
