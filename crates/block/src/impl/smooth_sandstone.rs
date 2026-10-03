use crate::block_definition::BlockDefinition;
use crate::block_id;
use crate::component::map_color_component::MapColorComponent;
use crate::component::mineable_component::MineableComponent;
use crate::const_block;

pub const SMOOTH_SANDSTONE: BlockDefinition = const_block! {
    identifier: block_id::SMOOTH_SANDSTONE,
    states: [],
    components: [
        MapColorComponent { r: 247, g: 233, b: 163, a: 255 },
        MineableComponent::hardness(2.0),
    ],
    permutations: [],
};
