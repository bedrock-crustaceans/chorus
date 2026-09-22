use crate::block::block_definition::BlockDefinition;
use crate::block::component::map_color_component::MapColorComponent;
use crate::block::component::mineable_component::MineableComponent;
use crate::const_block;

pub const LAPIS_BLOCK: BlockDefinition = const_block! {
    identifier: "minecraft:lapis_block",
    states: [],
    components: [
        MapColorComponent { r: 74, g: 128, b: 255, a: 255 },
        MineableComponent::hardness(3.0),
    ],
    permutations: [],
};
