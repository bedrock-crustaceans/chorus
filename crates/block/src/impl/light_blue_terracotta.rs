use crate::block_definition::BlockDefinition;
use crate::block_id;
use crate::component::map_color_component::MapColorComponent;
use crate::component::mineable_component::MineableComponent;
use crate::const_block;

pub const LIGHT_BLUE_TERRACOTTA: BlockDefinition = const_block! {
    identifier: block_id::LIGHT_BLUE_TERRACOTTA,
    states: [],
    components: [
        MapColorComponent { r: 112, g: 108, b: 138, a: 255 },
        MineableComponent::hardness(1.25),
    ],
    permutations: [],
};
