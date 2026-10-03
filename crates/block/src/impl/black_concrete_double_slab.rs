use crate::block_definition::BlockDefinition;
use crate::block_id;
use crate::component::map_color_component::MapColorComponent;
use crate::component::mineable_component::MineableComponent;
use crate::const_block;
use crate::state::common::MINECRAFT_VERTICAL_HALF;

pub const BLACK_CONCRETE_DOUBLE_SLAB: BlockDefinition = const_block! {
    identifier: block_id::BLACK_CONCRETE_DOUBLE_SLAB,
    states: [MINECRAFT_VERTICAL_HALF],
    components: [
        MapColorComponent { r: 25, g: 25, b: 25, a: 255 },
        MineableComponent::hardness(1.8),
    ],
    permutations: [],
};
