use crate::block::block_definition::BlockDefinition;
use crate::block::component::map_color_component::MapColorComponent;
use crate::block::component::mineable_component::MineableComponent;
use crate::block::state::common::MINECRAFT_VERTICAL_HALF;
use crate::const_block;

pub const POLISHED_BLACKSTONE_BRICK_DOUBLE_SLAB: BlockDefinition = const_block! {
    identifier: "minecraft:polished_blackstone_brick_double_slab",
    states: [MINECRAFT_VERTICAL_HALF],
    components: [
        MapColorComponent { r: 25, g: 25, b: 25, a: 255 },
        MineableComponent::hardness(2.0),
    ],
    permutations: [],
};
