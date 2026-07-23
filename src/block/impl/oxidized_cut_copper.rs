use crate::block::block_definition::BlockDefinition;
use crate::block::component::map_color_component::MapColorComponent;
use crate::block::component::mineable_component::MineableComponent;
use crate::const_block;

pub const OXIDIZED_CUT_COPPER: BlockDefinition = const_block! {
    identifier: "minecraft:oxidized_cut_copper",
    states: [],
    components: [
        MapColorComponent { r: 22, g: 126, b: 134, a: 255 },
        MineableComponent::hardness(3.0),
    ],
    permutations: [],
};
