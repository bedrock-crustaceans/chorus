use crate::block::block_definition::BlockDefinition;
use crate::block::block_id;
use crate::block::component::map_color_component::MapColorComponent;
use crate::block::component::mineable_component::MineableComponent;
use crate::const_block;

pub const CHEMICAL_HEAT: BlockDefinition = const_block! {
    identifier: block_id::CHEMICAL_HEAT,
    states: [],
    components: [
        MapColorComponent { r: 143, g: 119, b: 72, a: 255 },
        MineableComponent::hardness(2.5),
    ],
    permutations: [],
};
