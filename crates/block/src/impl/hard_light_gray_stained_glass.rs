use crate::block_definition::BlockDefinition;
use crate::block_id;
use crate::component::support_component::SupportComponent;
use crate::const_block;

pub static HARD_LIGHT_GRAY_STAINED_GLASS: BlockDefinition = const_block! {
    identifier: block_id::HARD_LIGHT_GRAY_STAINED_GLASS,
    states: [],
    components: [
        SupportComponent::ALL,
    ],
    permutations: [],
};
