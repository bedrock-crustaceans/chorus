use crate::block::block_definition::BlockDefinition;
use crate::block::block_id;
use crate::block::component::map_color_component::MapColorComponent;
use crate::block::component::mineable_component::MineableComponent;
use crate::const_block;

pub const ANCIENT_DEBRIS: BlockDefinition = const_block! {
    identifier: block_id::ANCIENT_DEBRIS,
    states: [],
    components: [
        MapColorComponent { r: 25, g: 25, b: 25, a: 255 },
        MineableComponent::hardness(30.0),
    ],
    permutations: [],
};
