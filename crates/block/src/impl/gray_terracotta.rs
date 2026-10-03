use crate::block_definition::BlockDefinition;
use crate::block_id;
use crate::component::map_color_component::MapColorComponent;
use crate::component::mineable_component::MineableComponent;
use crate::const_block;

pub const GRAY_TERRACOTTA: BlockDefinition = const_block! {
    identifier: block_id::GRAY_TERRACOTTA,
    states: [],
    components: [
        MapColorComponent { r: 57, g: 41, b: 35, a: 255 },
        MineableComponent::hardness(1.25),
    ],
    permutations: [],
};
