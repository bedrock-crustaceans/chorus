use crate::block::block_definition::BlockDefinition;
use crate::block::component::mineable_component::MineableComponent;
use crate::const_block;

pub const REDSTONE_LAMP: BlockDefinition = const_block! {
    identifier: "minecraft:redstone_lamp",
    states: [],
    components: [
        MineableComponent::hardness(0.3),
    ],
    permutations: [],
};
