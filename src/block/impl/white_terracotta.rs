use crate::block::block_definition::BlockDefinition;
use crate::block::component::map_color_component::MapColorComponent;
use crate::block::component::mineable_component::MineableComponent;
use crate::const_block;

pub const WHITE_TERRACOTTA: BlockDefinition = const_block! {
    identifier: "minecraft:white_terracotta",
    states: [],
    components: [
        MapColorComponent { r: 209, g: 177, b: 161, a: 255 },
        MineableComponent::hardness(1.25),
    ],
    permutations: [],
};
