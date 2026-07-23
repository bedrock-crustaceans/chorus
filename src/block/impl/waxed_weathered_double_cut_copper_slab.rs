use crate::block::block_definition::BlockDefinition;
use crate::block::component::map_color_component::MapColorComponent;
use crate::block::component::mineable_component::MineableComponent;
use crate::block::state::common::MINECRAFT_VERTICAL_HALF;
use crate::const_block;

pub const WAXED_WEATHERED_DOUBLE_CUT_COPPER_SLAB: BlockDefinition = const_block! {
    identifier: "minecraft:waxed_weathered_double_cut_copper_slab",
    states: [MINECRAFT_VERTICAL_HALF],
    components: [
        MapColorComponent { r: 58, g: 142, b: 140, a: 255 },
        MineableComponent::hardness(3.0),
    ],
    permutations: [],
};
