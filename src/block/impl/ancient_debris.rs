use crate::block::block_definition::BlockDefinition;
use crate::block::component::map_color_component::MapColorComponent;
use crate::block::component::mineable_component::MineableComponent;
use crate::const_block;

pub const ANCIENT_DEBRIS: BlockDefinition = const_block! {
    identifier: "minecraft:ancient_debris",
    states: [],
    components: [
        MapColorComponent { r: 25, g: 25, b: 25, a: 255 },
        MineableComponent::hardness(30.0),
    ],
    permutations: [],
};
