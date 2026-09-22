use crate::block::block_definition::BlockDefinition;
use crate::block::component::map_color_component::MapColorComponent;
use crate::block::component::mineable_component::MineableComponent;
use crate::const_block;

pub const CYAN_TERRACOTTA: BlockDefinition = const_block! {
    identifier: "minecraft:cyan_terracotta",
    states: [],
    components: [
        MapColorComponent { r: 87, g: 92, b: 92, a: 255 },
        MineableComponent::hardness(1.25),
    ],
    permutations: [],
};
