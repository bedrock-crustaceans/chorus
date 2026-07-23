use crate::block::block_definition::BlockDefinition;
use crate::block::component::map_color_component::MapColorComponent;
use crate::block::component::mineable_component::MineableComponent;
use crate::const_block;

pub const TUBE_CORAL_BLOCK: BlockDefinition = const_block! {
    identifier: "minecraft:tube_coral_block",
    states: [],
    components: [
        MapColorComponent { r: 51, g: 76, b: 178, a: 255 },
        MineableComponent::hardness(1.5),
    ],
    permutations: [],
};
