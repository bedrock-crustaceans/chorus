use crate::block::block_definition::BlockDefinition;
use crate::block::component::mineable_component::MineableComponent;
use crate::const_block;

pub const GRASS_BLOCK: BlockDefinition = const_block! {
    identifier: "minecraft:grass_block",
    states: [],
    components: [
        MineableComponent::hardness(0.6),
    ],
    permutations: [],
};
