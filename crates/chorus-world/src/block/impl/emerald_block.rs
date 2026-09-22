use crate::block::block_definition::BlockDefinition;
use crate::block::component::map_color_component::MapColorComponent;
use crate::block::component::mineable_component::MineableComponent;
use crate::const_block;

pub const EMERALD_BLOCK: BlockDefinition = const_block! {
    identifier: "minecraft:emerald_block",
    states: [],
    components: [
        MapColorComponent { r: 0, g: 217, b: 58, a: 255 },
        MineableComponent::hardness(5.0),
    ],
    permutations: [],
};
