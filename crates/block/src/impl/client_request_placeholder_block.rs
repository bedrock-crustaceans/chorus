use crate::block_definition::BlockDefinition;
use crate::block_id;
use crate::component::solid_component::SolidComponent;
use crate::component::support_component::SupportComponent;
use crate::const_block;

pub static CLIENT_REQUEST_PLACEHOLDER_BLOCK: BlockDefinition = const_block! {
    identifier: block_id::CLIENT_REQUEST_PLACEHOLDER_BLOCK,
    states: [],
    components: [
        SolidComponent { solid: false },
        SupportComponent::ALL,
    ],
    permutations: [],
};
