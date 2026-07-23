use crate::block::block_definition::BlockDefinition;
use crate::block::component::map_color_component::MapColorComponent;
use crate::block::component::mineable_component::MineableComponent;
use crate::const_block;

pub const YELLOW_CONCRETE: BlockDefinition = const_block! {
    identifier: "minecraft:yellow_concrete",
    states: [],
    components: [
        MapColorComponent { r: 229, g: 229, b: 51, a: 255 },
        MineableComponent::hardness(1.8),
    ],
    permutations: [],
};
