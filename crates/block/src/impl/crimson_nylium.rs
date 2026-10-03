use crate::block_definition::BlockDefinition;
use crate::block_id;
use crate::component::map_color_component::MapColorComponent;
use crate::component::mineable_component::MineableComponent;
use crate::const_block;

pub const CRIMSON_NYLIUM: BlockDefinition = const_block! {
    identifier: block_id::CRIMSON_NYLIUM,
    states: [],
    components: [
        MapColorComponent { r: 189, g: 48, b: 49, a: 255 },
        MineableComponent::hardness(0.4),
    ],
    permutations: [],
};
