use crate::block::block_definition::BlockDefinition;
use crate::block::component::map_color_component::MapColorComponent;
use crate::block::component::mineable_component::MineableComponent;
use crate::const_block;

pub const CRACKED_POLISHED_BLACKSTONE_BRICKS: BlockDefinition = const_block! {
    identifier: "minecraft:cracked_polished_blackstone_bricks",
    states: [],
    components: [
        MapColorComponent { r: 25, g: 25, b: 25, a: 255 },
        MineableComponent::hardness(1.5),
    ],
    permutations: [],
};
