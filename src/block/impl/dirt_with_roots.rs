use crate::block::block_definition::BlockDefinition;
use crate::block::component::map_color_component::MapColorComponent;
use crate::block::component::mineable_component::MineableComponent;
use crate::const_block;

pub const DIRT_WITH_ROOTS: BlockDefinition = const_block! {
    identifier: "minecraft:dirt_with_roots",
    states: [],
    components: [
        MapColorComponent { r: 151, g: 109, b: 77, a: 255 },
        MineableComponent::hardness(0.5),
    ],
    permutations: [],
};
