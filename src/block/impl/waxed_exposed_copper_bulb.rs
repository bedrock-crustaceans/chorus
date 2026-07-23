use crate::block::block_definition::BlockDefinition;
use crate::block::component::light_emission_component::LightEmissionComponent;
use crate::block::component::map_color_component::MapColorComponent;
use crate::block::component::mineable_component::MineableComponent;
use crate::block::state::common::{LIT, POWERED_BIT};
use crate::{const_block, const_permutation};

pub const WAXED_EXPOSED_COPPER_BULB: BlockDefinition = const_block! {
    identifier: "minecraft:waxed_exposed_copper_bulb",
    states: [LIT, POWERED_BIT],
    components: [
        MapColorComponent { r: 135, g: 107, b: 98, a: 255 },
        LightEmissionComponent { emission: 12 },
        MineableComponent::hardness(3.0),
    ],
    permutations: [
        const_permutation! {
            condition: |it| it["lit"] == false,
            components: [LightEmissionComponent { emission: 0 }]
        },
    ],
};
