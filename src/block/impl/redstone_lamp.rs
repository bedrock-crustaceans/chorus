use crate::block::block_definition::BlockDefinition;
use crate::block::block_id;
use crate::block::component::mineable_component::MineableComponent;
use crate::const_block;

pub const REDSTONE_LAMP: BlockDefinition = const_block! {
    identifier: block_id::REDSTONE_LAMP,
    states: [],
    components: [
        MineableComponent::hardness(0.3),
    ],
    permutations: [],
};
