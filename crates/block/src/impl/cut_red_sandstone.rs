use crate::block_definition::BlockDefinition;
use crate::block_id;
use crate::component::map_color_component::MapColorComponent;
use crate::component::mineable_component::MineableComponent;
use crate::const_block;

pub const CUT_RED_SANDSTONE: BlockDefinition = const_block! {
    identifier: block_id::CUT_RED_SANDSTONE,
    states: [],
    components: [
        MapColorComponent { r: 216, g: 127, b: 51, a: 255 },
        MineableComponent::hardness(0.8),
    ],
    permutations: [],
};
