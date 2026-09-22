use crate::block::block_definition::BlockDefinition;
use crate::block::component::map_color_component::MapColorComponent;
use crate::block::component::mineable_component::MineableComponent;
use crate::const_block;

pub const SCULK: BlockDefinition = const_block! {
    identifier: "minecraft:sculk",
    states: [],
    components: [
        MapColorComponent { r: 13, g: 18, b: 23, a: 255 },
        MineableComponent::hardness(0.2),
    ],
    permutations: [],
};
