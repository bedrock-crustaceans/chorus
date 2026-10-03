use crate::block::block_definition::BlockDefinition;
use crate::block::block_id;
use crate::block::component::map_color_component::MapColorComponent;
use crate::block::component::mineable_component::MineableComponent;
use crate::const_block;

pub const CLAY: BlockDefinition = const_block! {
    identifier: block_id::CLAY,
    states: [],
    components: [
        MapColorComponent { r: 164, g: 168, b: 184, a: 255 },
        MineableComponent::hardness(0.6),
    ],
    permutations: [],
};
