use crate::block::block_definition::BlockDefinition;
use crate::block::component::map_color_component::MapColorComponent;
use crate::block::component::mineable_component::MineableComponent;
use crate::const_block;

pub const NETHER_GOLD_ORE: BlockDefinition = const_block! {
    identifier: "minecraft:nether_gold_ore",
    states: [],
    components: [
        MapColorComponent { r: 112, g: 2, b: 0, a: 255 },
        MineableComponent::hardness(3.0),
    ],
    permutations: [],
};
