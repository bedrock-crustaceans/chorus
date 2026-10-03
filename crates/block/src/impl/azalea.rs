use crate::block_definition::BlockDefinition;
use crate::block_id;
use crate::component::map_color_component::MapColorComponent;
use crate::component::mineable_component::MineableComponent;
use crate::const_block;

pub const AZALEA: BlockDefinition = const_block! {
    identifier: block_id::AZALEA,
    states: [],
    components: [
        MapColorComponent { r: 0, g: 124, b: 0, a: 255 },
        MineableComponent::hardness(0.0),
    ],
    permutations: [],
};
