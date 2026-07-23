use crate::block::block_definition::BlockDefinition;
use crate::block::component::light_emission_component::LightEmissionComponent;
use crate::block::component::map_color_component::MapColorComponent;
use crate::block::component::mineable_component::MineableComponent;
use crate::block::state::common::{LIT, POWERED_BIT};
use crate::{const_block, const_permutation};

pub const WEATHERED_COPPER_BULB: BlockDefinition = const_block! {
    identifier: "minecraft:weathered_copper_bulb",
    states: [LIT, POWERED_BIT],
    components: [
        MapColorComponent { r: 58, g: 142, b: 140, a: 255 },
        LightEmissionComponent { emission: 8 },
        MineableComponent::hardness(3.0),
    ],
    permutations: [
        const_permutation! {
            condition: |it| it["lit"] == false,
            components: [LightEmissionComponent { emission: 0 }]
        },
    ],
};
