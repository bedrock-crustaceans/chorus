use crate::block::block_definition::BlockDefinition;
use crate::block::component::map_color_component::MapColorComponent;
use crate::block::component::mineable_component::MineableComponent;
use crate::const_block;

pub const PODZOL: BlockDefinition = const_block! {
    identifier: "minecraft:podzol",
    states: [],
    components: [
        MapColorComponent { r: 129, g: 86, b: 49, a: 255 },
        MineableComponent::hardness(0.6),
    ],
    permutations: [],
};
