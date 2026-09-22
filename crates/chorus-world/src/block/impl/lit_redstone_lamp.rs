use crate::block::block_definition::BlockDefinition;
use crate::block::component::light_emission_component::LightEmissionComponent;
use crate::block::component::mineable_component::MineableComponent;
use crate::const_block;

pub const LIT_REDSTONE_LAMP: BlockDefinition = const_block! {
    identifier: "minecraft:lit_redstone_lamp",
    states: [],
    components: [
        LightEmissionComponent { emission: 15 },
        MineableComponent::hardness(0.3),
    ],
    permutations: [],
};
