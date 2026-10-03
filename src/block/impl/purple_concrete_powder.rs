use crate::block::block_definition::BlockDefinition;
use crate::block::block_id;
use crate::block::component::map_color_component::MapColorComponent;
use crate::block::component::mineable_component::MineableComponent;
use crate::const_block;

pub const PURPLE_CONCRETE_POWDER: BlockDefinition = const_block! {
    identifier: block_id::PURPLE_CONCRETE_POWDER,
    states: [],
    components: [
        MapColorComponent { r: 153, g: 90, b: 205, a: 255 },
        MineableComponent::hardness(0.5),
    ],
    permutations: [],
};
