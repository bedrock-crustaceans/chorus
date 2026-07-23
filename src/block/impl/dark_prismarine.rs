use crate::block::block_definition::BlockDefinition;
use crate::block::component::map_color_component::MapColorComponent;
use crate::block::component::mineable_component::MineableComponent;
use crate::const_block;

pub const DARK_PRISMARINE: BlockDefinition = const_block! {
    identifier: "minecraft:dark_prismarine",
    states: [],
    components: [
        MapColorComponent { r: 92, g: 219, b: 213, a: 255 },
        MineableComponent::hardness(1.5),
    ],
    permutations: [],
};
