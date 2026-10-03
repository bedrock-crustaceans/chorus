use crate::block_definition::BlockDefinition;
use crate::block_id;
use crate::component::map_color_component::MapColorComponent;
use crate::component::mineable_component::MineableComponent;
use crate::const_block;

pub const POLISHED_BLACKSTONE: BlockDefinition = const_block! {
    identifier: block_id::POLISHED_BLACKSTONE,
    states: [],
    components: [
        MapColorComponent { r: 25, g: 25, b: 25, a: 255 },
        MineableComponent::hardness(1.5),
    ],
    permutations: [],
};
