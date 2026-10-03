use crate::block_definition::BlockDefinition;
use crate::block_id;
use crate::component::map_color_component::MapColorComponent;
use crate::component::mineable_component::MineableComponent;
use crate::const_block;

pub const DEAD_FIRE_CORAL_BLOCK: BlockDefinition = const_block! {
    identifier: block_id::DEAD_FIRE_CORAL_BLOCK,
    states: [],
    components: [
        MapColorComponent { r: 76, g: 76, b: 76, a: 255 },
        MineableComponent::hardness(1.5),
    ],
    permutations: [],
};
