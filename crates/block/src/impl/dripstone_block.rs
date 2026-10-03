use crate::block_definition::BlockDefinition;
use crate::block_id;
use crate::component::map_color_component::MapColorComponent;
use crate::component::mineable_component::MineableComponent;
use crate::const_block;

pub const DRIPSTONE_BLOCK: BlockDefinition = const_block! {
    identifier: block_id::DRIPSTONE_BLOCK,
    states: [],
    components: [
        MapColorComponent { r: 76, g: 50, b: 35, a: 255 },
        MineableComponent::hardness(1.5),
    ],
    permutations: [],
};
