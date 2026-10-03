use crate::block_definition::BlockDefinition;
use crate::block_id;
use crate::component::map_color_component::MapColorComponent;
use crate::component::mineable_component::MineableComponent;
use crate::const_block;

pub const GRAY_CONCRETE: BlockDefinition = const_block! {
    identifier: block_id::GRAY_CONCRETE,
    states: [],
    components: [
        MapColorComponent { r: 76, g: 76, b: 76, a: 255 },
        MineableComponent::hardness(1.8),
    ],
    permutations: [],
};
