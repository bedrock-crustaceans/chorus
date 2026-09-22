use crate::block::block_definition::BlockDefinition;
use crate::block::component::map_color_component::MapColorComponent;
use crate::block::component::mineable_component::MineableComponent;
use crate::const_block;

pub const PRISMARINE_BRICKS: BlockDefinition = const_block! {
    identifier: "minecraft:prismarine_bricks",
    states: [],
    components: [
        MapColorComponent { r: 92, g: 219, b: 213, a: 255 },
        MineableComponent::hardness(1.5),
    ],
    permutations: [],
};
