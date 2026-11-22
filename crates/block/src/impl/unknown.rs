use crate::block_definition::BlockDefinition;
use crate::block_id;
use crate::component::support_component::SupportComponent;
use crate::const_block;

pub static UNKNOWN: BlockDefinition = const_block! {
    identifier: block_id::UNKNOWN,
    states: [],
    components: [
        SupportComponent::ALL,
    ],
    permutations: [],
};
