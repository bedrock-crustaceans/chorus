use crate::block::block_definition::BlockDefinition;
use crate::block::block_id;
use crate::block::component::map_color_component::MapColorComponent;
use crate::block::component::mineable_component::MineableComponent;
use crate::const_block;

pub const PRISMARINE: BlockDefinition = const_block! {
    identifier: block_id::PRISMARINE,
    states: [],
    components: [
        MapColorComponent { r: 76, g: 127, b: 153, a: 255 },
        MineableComponent::hardness(1.5),
    ],
    permutations: [],
};
