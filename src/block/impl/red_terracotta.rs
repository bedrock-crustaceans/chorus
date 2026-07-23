use crate::block::block_definition::BlockDefinition;
use crate::block::component::map_color_component::MapColorComponent;
use crate::block::component::mineable_component::MineableComponent;
use crate::const_block;

pub const RED_TERRACOTTA: BlockDefinition = const_block! {
    identifier: "minecraft:red_terracotta",
    states: [],
    components: [
        MapColorComponent { r: 142, g: 60, b: 46, a: 255 },
        MineableComponent::hardness(1.25),
    ],
    permutations: [],
};
