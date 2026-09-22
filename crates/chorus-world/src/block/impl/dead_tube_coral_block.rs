use crate::block::block_definition::BlockDefinition;
use crate::block::component::map_color_component::MapColorComponent;
use crate::block::component::mineable_component::MineableComponent;
use crate::const_block;

pub const DEAD_TUBE_CORAL_BLOCK: BlockDefinition = const_block! {
    identifier: "minecraft:dead_tube_coral_block",
    states: [],
    components: [
        MapColorComponent { r: 76, g: 76, b: 76, a: 255 },
        MineableComponent::hardness(1.5),
    ],
    permutations: [],
};
