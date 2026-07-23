use crate::block::block_definition::BlockDefinition;
use crate::block::component::map_color_component::MapColorComponent;
use crate::block::component::mineable_component::MineableComponent;
use crate::const_block;

pub const HORN_CORAL_BLOCK: BlockDefinition = const_block! {
    identifier: "minecraft:horn_coral_block",
    states: [],
    components: [
        MapColorComponent { r: 229, g: 229, b: 51, a: 255 },
        MineableComponent::hardness(1.5),
    ],
    permutations: [],
};
