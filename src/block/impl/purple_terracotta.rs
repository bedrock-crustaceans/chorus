use crate::block::block_definition::BlockDefinition;
use crate::block::block_id;
use crate::block::component::map_color_component::MapColorComponent;
use crate::block::component::mineable_component::MineableComponent;
use crate::const_block;

pub const PURPLE_TERRACOTTA: BlockDefinition = const_block! {
    identifier: block_id::PURPLE_TERRACOTTA,
    states: [],
    components: [
        MapColorComponent { r: 122, g: 73, b: 88, a: 255 },
        MineableComponent::hardness(1.25),
    ],
    permutations: [],
};
