use crate::block_definition::BlockDefinition;
use crate::block_id;
use crate::component::map_color_component::MapColorComponent;
use crate::component::mineable_component::MineableComponent;
use crate::const_block;

pub const WHITE_CONCRETE_POWDER: BlockDefinition = const_block! {
    identifier: block_id::WHITE_CONCRETE_POWDER,
    states: [],
    components: [
        MapColorComponent { r: 255, g: 255, b: 255, a: 255 },
        MineableComponent::hardness(0.5),
    ],
    permutations: [],
};
