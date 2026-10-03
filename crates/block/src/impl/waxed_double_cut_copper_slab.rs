use crate::block_definition::BlockDefinition;
use crate::block_id;
use crate::component::map_color_component::MapColorComponent;
use crate::component::mineable_component::MineableComponent;
use crate::const_block;
use crate::state::common::MINECRAFT_VERTICAL_HALF;

pub const WAXED_DOUBLE_CUT_COPPER_SLAB: BlockDefinition = const_block! {
    identifier: block_id::WAXED_DOUBLE_CUT_COPPER_SLAB,
    states: [MINECRAFT_VERTICAL_HALF],
    components: [
        MapColorComponent { r: 216, g: 127, b: 51, a: 255 },
        MineableComponent::hardness(3.0),
    ],
    permutations: [],
};
