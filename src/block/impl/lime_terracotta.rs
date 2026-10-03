use crate::block::block_definition::BlockDefinition;
use crate::block::block_id;
use crate::block::component::map_color_component::MapColorComponent;
use crate::block::component::mineable_component::MineableComponent;
use crate::const_block;

pub const LIME_TERRACOTTA: BlockDefinition = const_block! {
    identifier: block_id::LIME_TERRACOTTA,
    states: [],
    components: [
        MapColorComponent { r: 103, g: 117, b: 53, a: 255 },
        MineableComponent::hardness(1.25),
    ],
    permutations: [],
};
