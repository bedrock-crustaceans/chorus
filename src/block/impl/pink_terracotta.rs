use crate::block::block_definition::BlockDefinition;
use crate::block::component::map_color_component::MapColorComponent;
use crate::block::component::mineable_component::MineableComponent;
use crate::const_block;

pub const PINK_TERRACOTTA: BlockDefinition = const_block! {
    identifier: "minecraft:pink_terracotta",
    states: [],
    components: [
        MapColorComponent { r: 160, g: 77, b: 78, a: 255 },
        MineableComponent::hardness(1.25),
    ],
    permutations: [],
};
