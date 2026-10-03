use crate::block_definition::BlockDefinition;
use crate::block_id;
use crate::component::map_color_component::MapColorComponent;
use crate::component::mineable_component::MineableComponent;
use crate::const_block;

pub const BUBBLE_CORAL_BLOCK: BlockDefinition = const_block! {
    identifier: block_id::BUBBLE_CORAL_BLOCK,
    states: [],
    components: [
        MapColorComponent { r: 153, g: 90, b: 205, a: 255 },
        MineableComponent::hardness(1.5),
    ],
    permutations: [],
};
