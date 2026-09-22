use crate::block::block_definition::BlockDefinition;
use crate::block::component::map_color_component::MapColorComponent;
use crate::block::component::mineable_component::MineableComponent;
use crate::const_block;

pub const WEATHERED_COPPER: BlockDefinition = const_block! {
    identifier: "minecraft:weathered_copper",
    states: [],
    components: [
        MapColorComponent { r: 58, g: 142, b: 140, a: 255 },
        MineableComponent::hardness(3.0),
    ],
    permutations: [],
};
