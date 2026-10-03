use crate::block_definition::BlockDefinition;
use crate::block_id;
use crate::component::light_emission_component::LightEmissionComponent;
use crate::component::mineable_component::MineableComponent;
use crate::const_block;

pub const LIT_REDSTONE_LAMP: BlockDefinition = const_block! {
    identifier: block_id::LIT_REDSTONE_LAMP,
    states: [],
    components: [
        LightEmissionComponent { emission: 15 },
        MineableComponent::hardness(0.3),
    ],
    permutations: [],
};
