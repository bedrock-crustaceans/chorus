use crate::block_definition::BlockDefinition;
use crate::block_id;
use crate::component::map_color_component::MapColorComponent;
use crate::component::mineable_component::MineableComponent;
use crate::const_block;

pub const PODZOL: BlockDefinition = const_block! {
    identifier: block_id::PODZOL,
    states: [],
    components: [
        MapColorComponent { r: 129, g: 86, b: 49, a: 255 },
        MineableComponent::hardness(0.6),
    ],
    permutations: [],
};
