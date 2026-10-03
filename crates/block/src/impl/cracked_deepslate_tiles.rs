use crate::block_definition::BlockDefinition;
use crate::block_id;
use crate::component::map_color_component::MapColorComponent;
use crate::component::mineable_component::MineableComponent;
use crate::const_block;

pub const CRACKED_DEEPSLATE_TILES: BlockDefinition = const_block! {
    identifier: block_id::CRACKED_DEEPSLATE_TILES,
    states: [],
    components: [
        MapColorComponent { r: 100, g: 100, b: 100, a: 255 },
        MineableComponent::hardness(3.5),
    ],
    permutations: [],
};
