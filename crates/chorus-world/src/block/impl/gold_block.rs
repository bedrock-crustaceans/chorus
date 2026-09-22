use crate::block::block_definition::BlockDefinition;
use crate::block::component::map_color_component::MapColorComponent;
use crate::block::component::mineable_component::MineableComponent;
use crate::const_block;

pub const GOLD_BLOCK: BlockDefinition = const_block! {
    identifier: "minecraft:gold_block",
    states: [],
    components: [
        MapColorComponent { r: 250, g: 238, b: 77, a: 255 },
        MineableComponent::hardness(3.0),
    ],
    permutations: [],
};
