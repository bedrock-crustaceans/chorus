use crate::block::block_definition::BlockDefinition;
use crate::block::component::map_color_component::MapColorComponent;
use crate::block::component::mineable_component::MineableComponent;
use crate::block::state::common::MINECRAFT_VERTICAL_HALF;
use crate::const_block;

pub const SPRUCE_DOUBLE_SLAB: BlockDefinition = const_block! {
    identifier: "minecraft:spruce_double_slab",
    states: [MINECRAFT_VERTICAL_HALF],
    components: [
        MapColorComponent { r: 129, g: 86, b: 49, a: 255 },
        MineableComponent::hardness(2.0),
    ],
    permutations: [],
};
