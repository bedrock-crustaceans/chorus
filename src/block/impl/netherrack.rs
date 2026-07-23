use crate::block::block_definition::BlockDefinition;
use crate::block::component::map_color_component::MapColorComponent;
use crate::block::component::mineable_component::MineableComponent;
use crate::const_block;

pub const NETHERRACK: BlockDefinition = const_block! {
    identifier: "minecraft:netherrack",
    states: [],
    components: [
        MapColorComponent { r: 112, g: 2, b: 0, a: 255 },
        MineableComponent::hardness(0.4),
    ],
    permutations: [],
};
