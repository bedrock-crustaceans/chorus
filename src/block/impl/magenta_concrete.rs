use crate::block::block_definition::BlockDefinition;
use crate::block::component::map_color_component::MapColorComponent;
use crate::block::component::mineable_component::MineableComponent;
use crate::const_block;

pub const MAGENTA_CONCRETE: BlockDefinition = const_block! {
    identifier: "minecraft:magenta_concrete",
    states: [],
    components: [
        MapColorComponent { r: 178, g: 76, b: 216, a: 255 },
        MineableComponent::hardness(1.8),
    ],
    permutations: [],
};
