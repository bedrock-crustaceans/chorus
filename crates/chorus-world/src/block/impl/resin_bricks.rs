use crate::block::block_definition::BlockDefinition;
use crate::block::component::map_color_component::MapColorComponent;
use crate::block::component::mineable_component::MineableComponent;
use crate::const_block;

pub const RESIN_BRICKS: BlockDefinition = const_block! {
    identifier: "minecraft:resin_bricks",
    states: [],
    components: [
        MapColorComponent { r: 159, g: 82, b: 36, a: 255 },
        MineableComponent::hardness(1.5),
    ],
    permutations: [],
};
