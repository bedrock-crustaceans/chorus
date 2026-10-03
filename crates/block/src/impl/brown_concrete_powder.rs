use crate::block_definition::BlockDefinition;
use crate::block_id;
use crate::component::map_color_component::MapColorComponent;
use crate::component::mineable_component::MineableComponent;
use crate::const_block;

pub const BROWN_CONCRETE_POWDER: BlockDefinition = const_block! {
    identifier: block_id::BROWN_CONCRETE_POWDER,
    states: [],
    components: [
        MapColorComponent { r: 102, g: 76, b: 51, a: 255 },
        MineableComponent::hardness(0.5),
    ],
    permutations: [],
};
