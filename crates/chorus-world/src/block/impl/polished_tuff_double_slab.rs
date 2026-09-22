use crate::block::block_definition::BlockDefinition;
use crate::block::component::map_color_component::MapColorComponent;
use crate::block::component::mineable_component::MineableComponent;
use crate::block::state::common::MINECRAFT_VERTICAL_HALF;
use crate::const_block;

pub const POLISHED_TUFF_DOUBLE_SLAB: BlockDefinition = const_block! {
    identifier: "minecraft:polished_tuff_double_slab",
    states: [MINECRAFT_VERTICAL_HALF],
    components: [
        MapColorComponent { r: 57, g: 41, b: 35, a: 255 },
        MineableComponent::hardness(1.5),
    ],
    permutations: [],
};
