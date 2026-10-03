use crate::block_definition::BlockDefinition;
use crate::block_id;
use crate::component::map_color_component::MapColorComponent;
use crate::component::mineable_component::MineableComponent;
use crate::const_block;

pub const YELLOW_TERRACOTTA: BlockDefinition = const_block! {
    identifier: block_id::YELLOW_TERRACOTTA,
    states: [],
    components: [
        MapColorComponent { r: 186, g: 133, b: 36, a: 255 },
        MineableComponent::hardness(1.25),
    ],
    permutations: [],
};
