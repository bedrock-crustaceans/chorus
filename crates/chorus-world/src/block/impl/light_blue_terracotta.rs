use crate::block::block_definition::BlockDefinition;
use crate::block::component::map_color_component::MapColorComponent;
use crate::block::component::mineable_component::MineableComponent;
use crate::const_block;

pub const LIGHT_BLUE_TERRACOTTA: BlockDefinition = const_block! {
    identifier: "minecraft:light_blue_terracotta",
    states: [],
    components: [
        MapColorComponent { r: 112, g: 108, b: 138, a: 255 },
        MineableComponent::hardness(1.25),
    ],
    permutations: [],
};
