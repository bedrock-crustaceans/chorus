use crate::block::block_definition::BlockDefinition;
use crate::block::block_id;
use crate::block::component::map_color_component::MapColorComponent;
use crate::block::component::mineable_component::MineableComponent;
use crate::block::state::common::MINECRAFT_VERTICAL_HALF;
use crate::const_block;

pub const PINK_CONCRETE_DOUBLE_SLAB: BlockDefinition = const_block! {
    identifier: block_id::PINK_CONCRETE_DOUBLE_SLAB,
    states: [MINECRAFT_VERTICAL_HALF],
    components: [
        MapColorComponent { r: 242, g: 127, b: 165, a: 255 },
        MineableComponent::hardness(1.8),
    ],
    permutations: [],
};
