use crate::block::block_definition::BlockDefinition;
use crate::block::component::map_color_component::MapColorComponent;
use crate::block::component::mineable_component::MineableComponent;
use crate::const_block;

pub const MOSS_BLOCK: BlockDefinition = const_block! {
    identifier: "minecraft:moss_block",
    states: [],
    components: [
        MapColorComponent { r: 102, g: 127, b: 51, a: 255 },
        MineableComponent::hardness(0.1),
    ],
    permutations: [],
};
