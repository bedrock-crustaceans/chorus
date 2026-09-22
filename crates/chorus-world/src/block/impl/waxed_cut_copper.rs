use crate::block::block_definition::BlockDefinition;
use crate::block::component::map_color_component::MapColorComponent;
use crate::block::component::mineable_component::MineableComponent;
use crate::const_block;

pub const WAXED_CUT_COPPER: BlockDefinition = const_block! {
    identifier: "minecraft:waxed_cut_copper",
    states: [],
    components: [
        MapColorComponent { r: 216, g: 127, b: 51, a: 255 },
        MineableComponent::hardness(3.0),
    ],
    permutations: [],
};
