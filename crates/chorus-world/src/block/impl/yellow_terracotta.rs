use crate::block::block_definition::BlockDefinition;
use crate::block::component::map_color_component::MapColorComponent;
use crate::block::component::mineable_component::MineableComponent;
use crate::const_block;

pub const YELLOW_TERRACOTTA: BlockDefinition = const_block! {
    identifier: "minecraft:yellow_terracotta",
    states: [],
    components: [
        MapColorComponent { r: 186, g: 133, b: 36, a: 255 },
        MineableComponent::hardness(1.25),
    ],
    permutations: [],
};
