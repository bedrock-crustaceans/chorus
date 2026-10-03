use crate::block::block_definition::BlockDefinition;
use crate::block::block_id;
use crate::block::component::map_color_component::MapColorComponent;
use crate::block::component::mineable_component::MineableComponent;
use crate::const_block;

pub const BRAIN_CORAL_BLOCK: BlockDefinition = const_block! {
    identifier: block_id::BRAIN_CORAL_BLOCK,
    states: [],
    components: [
        MapColorComponent { r: 242, g: 127, b: 165, a: 255 },
        MineableComponent::hardness(1.5),
    ],
    permutations: [],
};
