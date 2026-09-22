use crate::block::block_definition::BlockDefinition;
use crate::block::component::map_color_component::MapColorComponent;
use crate::block::component::mineable_component::MineableComponent;
use crate::const_block;

pub const PURPLE_TERRACOTTA: BlockDefinition = const_block! {
    identifier: "minecraft:purple_terracotta",
    states: [],
    components: [
        MapColorComponent { r: 122, g: 73, b: 88, a: 255 },
        MineableComponent::hardness(1.25),
    ],
    permutations: [],
};
