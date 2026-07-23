use crate::block::block_definition::BlockDefinition;
use crate::block::component::map_color_component::MapColorComponent;
use crate::block::component::mineable_component::MineableComponent;
use crate::const_block;

pub const WARPED_NYLIUM: BlockDefinition = const_block! {
    identifier: "minecraft:warped_nylium",
    states: [],
    components: [
        MapColorComponent { r: 22, g: 126, b: 134, a: 255 },
        MineableComponent::hardness(0.4),
    ],
    permutations: [],
};
