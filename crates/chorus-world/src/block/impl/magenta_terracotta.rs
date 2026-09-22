use crate::block::block_definition::BlockDefinition;
use crate::block::component::map_color_component::MapColorComponent;
use crate::block::component::mineable_component::MineableComponent;
use crate::const_block;

pub const MAGENTA_TERRACOTTA: BlockDefinition = const_block! {
    identifier: "minecraft:magenta_terracotta",
    states: [],
    components: [
        MapColorComponent { r: 149, g: 87, b: 108, a: 255 },
        MineableComponent::hardness(1.25),
    ],
    permutations: [],
};
