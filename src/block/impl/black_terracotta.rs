use crate::block::block_definition::BlockDefinition;
use crate::block::component::map_color_component::MapColorComponent;
use crate::block::component::mineable_component::MineableComponent;
use crate::const_block;

pub const BLACK_TERRACOTTA: BlockDefinition = const_block! {
    identifier: "minecraft:black_terracotta",
    states: [],
    components: [
        MapColorComponent { r: 37, g: 22, b: 16, a: 255 },
        MineableComponent::hardness(1.25),
    ],
    permutations: [],
};
