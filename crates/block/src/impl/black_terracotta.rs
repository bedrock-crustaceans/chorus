use crate::block_definition::BlockDefinition;
use crate::block_id;
use crate::component::map_color_component::MapColorComponent;
use crate::component::mineable_component::MineableComponent;
use crate::const_block;

pub const BLACK_TERRACOTTA: BlockDefinition = const_block! {
    identifier: block_id::BLACK_TERRACOTTA,
    states: [],
    components: [
        MapColorComponent { r: 37, g: 22, b: 16, a: 255 },
        MineableComponent::hardness(1.25),
    ],
    permutations: [],
};
