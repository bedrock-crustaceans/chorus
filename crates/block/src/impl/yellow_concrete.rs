use crate::block_definition::BlockDefinition;
use crate::block_id;
use crate::component::map_color_component::MapColorComponent;
use crate::component::mineable_component::MineableComponent;
use crate::const_block;

pub const YELLOW_CONCRETE: BlockDefinition = const_block! {
    identifier: block_id::YELLOW_CONCRETE,
    states: [],
    components: [
        MapColorComponent { r: 229, g: 229, b: 51, a: 255 },
        MineableComponent::hardness(1.8),
    ],
    permutations: [],
};
