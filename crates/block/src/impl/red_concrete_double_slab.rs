use crate::block_definition::BlockDefinition;
use crate::block_id;
use crate::component::map_color_component::MapColorComponent;
use crate::component::mineable_component::MineableComponent;
use crate::const_block;
use crate::state::common::MINECRAFT_VERTICAL_HALF;

pub const RED_CONCRETE_DOUBLE_SLAB: BlockDefinition = const_block! {
    identifier: block_id::RED_CONCRETE_DOUBLE_SLAB,
    states: [MINECRAFT_VERTICAL_HALF],
    components: [
        MapColorComponent { r: 153, g: 51, b: 51, a: 255 },
        MineableComponent::hardness(1.8),
    ],
    permutations: [],
};
