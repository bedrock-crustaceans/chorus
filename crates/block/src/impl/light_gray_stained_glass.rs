use crate::block_definition::BlockDefinition;
use crate::block_id;
use crate::component::support_component::SupportComponent;
use crate::const_block;

pub static LIGHT_GRAY_STAINED_GLASS: BlockDefinition = const_block! {
    identifier: block_id::LIGHT_GRAY_STAINED_GLASS,
    states: [],
    components: [
        SupportComponent::ALL,
    ],
    permutations: [],
};
