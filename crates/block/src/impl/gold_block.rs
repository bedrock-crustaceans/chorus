use crate::block_definition::BlockDefinition;
use crate::block_id;
use crate::component::map_color_component::MapColorComponent;
use crate::component::mineable_component::MineableComponent;
use crate::const_block;

pub const GOLD_BLOCK: BlockDefinition = const_block! {
    identifier: block_id::GOLD_BLOCK,
    states: [],
    components: [
        MapColorComponent { r: 250, g: 238, b: 77, a: 255 },
        MineableComponent::hardness(3.0),
    ],
    permutations: [],
};
