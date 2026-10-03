use crate::block_definition::BlockDefinition;
use crate::block_id;
use crate::component::map_color_component::MapColorComponent;
use crate::component::mineable_component::MineableComponent;
use crate::const_block;

pub const OXIDIZED_CHISELED_COPPER: BlockDefinition = const_block! {
    identifier: block_id::OXIDIZED_CHISELED_COPPER,
    states: [],
    components: [
        MapColorComponent { r: 22, g: 126, b: 134, a: 255 },
        MineableComponent::hardness(3.0),
    ],
    permutations: [],
};
