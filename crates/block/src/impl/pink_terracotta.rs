use crate::block_definition::BlockDefinition;
use crate::block_id;
use crate::component::map_color_component::MapColorComponent;
use crate::component::mineable_component::MineableComponent;
use crate::const_block;

pub const PINK_TERRACOTTA: BlockDefinition = const_block! {
    identifier: block_id::PINK_TERRACOTTA,
    states: [],
    components: [
        MapColorComponent { r: 160, g: 77, b: 78, a: 255 },
        MineableComponent::hardness(1.25),
    ],
    permutations: [],
};
