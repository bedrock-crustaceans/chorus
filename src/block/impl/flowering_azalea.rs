use crate::block::block_definition::BlockDefinition;
use crate::block::block_id;
use crate::block::component::map_color_component::MapColorComponent;
use crate::block::component::mineable_component::MineableComponent;
use crate::const_block;

pub const FLOWERING_AZALEA: BlockDefinition = const_block! {
    identifier: block_id::FLOWERING_AZALEA,
    states: [],
    components: [
        MapColorComponent { r: 0, g: 124, b: 0, a: 255 },
        MineableComponent::hardness(0.0),
    ],
    permutations: [],
};
