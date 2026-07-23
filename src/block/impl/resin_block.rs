use crate::block::block_definition::BlockDefinition;
use crate::block::component::map_color_component::MapColorComponent;
use crate::block::component::mineable_component::MineableComponent;
use crate::const_block;

pub const RESIN_BLOCK: BlockDefinition = const_block! {
    identifier: "minecraft:resin_block",
    states: [],
    components: [
        MapColorComponent { r: 159, g: 82, b: 36, a: 255 },
        MineableComponent::hardness(0.0),
    ],
    permutations: [],
};
