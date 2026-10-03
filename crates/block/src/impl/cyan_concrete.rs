use crate::block_definition::BlockDefinition;
use crate::block_id;
use crate::component::map_color_component::MapColorComponent;
use crate::component::mineable_component::MineableComponent;
use crate::const_block;

pub const CYAN_CONCRETE: BlockDefinition = const_block! {
    identifier: block_id::CYAN_CONCRETE,
    states: [],
    components: [
        MapColorComponent { r: 76, g: 127, b: 153, a: 255 },
        MineableComponent::hardness(1.8),
    ],
    permutations: [],
};
