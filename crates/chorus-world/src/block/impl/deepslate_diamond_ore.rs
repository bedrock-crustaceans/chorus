use crate::block::block_definition::BlockDefinition;
use crate::block::component::map_color_component::MapColorComponent;
use crate::block::component::mineable_component::MineableComponent;
use crate::const_block;

pub const DEEPSLATE_DIAMOND_ORE: BlockDefinition = const_block! {
    identifier: "minecraft:deepslate_diamond_ore",
    states: [],
    components: [
        MapColorComponent { r: 100, g: 100, b: 100, a: 255 },
        MineableComponent::hardness(4.5),
    ],
    permutations: [],
};
