use crate::block::block_definition::BlockDefinition;
use crate::block::component::map_color_component::MapColorComponent;
use crate::block::component::mineable_component::MineableComponent;
use crate::const_block;

pub const CHISELED_DEEPSLATE: BlockDefinition = const_block! {
    identifier: "minecraft:chiseled_deepslate",
    states: [],
    components: [
        MapColorComponent { r: 100, g: 100, b: 100, a: 255 },
        MineableComponent::hardness(3.5),
    ],
    permutations: [],
};
