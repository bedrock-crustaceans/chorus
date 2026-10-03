use crate::block::block_definition::BlockDefinition;
use crate::block::block_id;
use crate::block::component::map_color_component::MapColorComponent;
use crate::block::component::mineable_component::MineableComponent;
use crate::const_block;

pub const EXPOSED_CUT_COPPER: BlockDefinition = const_block! {
    identifier: block_id::EXPOSED_CUT_COPPER,
    states: [],
    components: [
        MapColorComponent { r: 135, g: 107, b: 98, a: 255 },
        MineableComponent::hardness(3.0),
    ],
    permutations: [],
};
