use crate::block::block_definition::BlockDefinition;
use crate::block::block_id;
use crate::block::component::map_color_component::MapColorComponent;
use crate::block::component::mineable_component::MineableComponent;
use crate::block::state::common::MINECRAFT_VERTICAL_HALF;
use crate::const_block;

pub const WAXED_DOUBLE_CUT_COPPER_SLAB: BlockDefinition = const_block! {
    identifier: block_id::WAXED_DOUBLE_CUT_COPPER_SLAB,
    states: [MINECRAFT_VERTICAL_HALF],
    components: [
        MapColorComponent { r: 216, g: 127, b: 51, a: 255 },
        MineableComponent::hardness(3.0),
    ],
    permutations: [],
};
