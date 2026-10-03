use crate::block_definition::BlockDefinition;
use crate::block_id;
use crate::component::map_color_component::MapColorComponent;
use crate::component::mineable_component::MineableComponent;
use crate::const_block;

pub const WAXED_OXIDIZED_COPPER: BlockDefinition = const_block! {
    identifier: block_id::WAXED_OXIDIZED_COPPER,
    states: [],
    components: [
        MapColorComponent { r: 22, g: 126, b: 134, a: 255 },
        MineableComponent::hardness(3.0),
    ],
    permutations: [],
};
