use crate::block::block_definition::BlockDefinition;
use crate::block::component::map_color_component::MapColorComponent;
use crate::block::component::mineable_component::MineableComponent;
use crate::block::state::common::MINECRAFT_VERTICAL_HALF;
use crate::const_block;

pub const PRISMARINE_DOUBLE_SLAB: BlockDefinition = const_block! {
    identifier: "minecraft:prismarine_double_slab",
    states: [MINECRAFT_VERTICAL_HALF],
    components: [
        MapColorComponent { r: 76, g: 127, b: 153, a: 255 },
        MineableComponent::hardness(2.0),
    ],
    permutations: [],
};
