use crate::block::block_definition::BlockDefinition;
use crate::block::component::light_emission_component::LightEmissionComponent;
use crate::block::component::map_color_component::MapColorComponent;
use crate::block::component::mineable_component::MineableComponent;
use crate::block::state::common::{LIT, POWERED_BIT};
use crate::{const_block, const_permutation};

pub const COPPER_BULB: BlockDefinition = const_block! {
    identifier: "minecraft:copper_bulb",
    states: [LIT, POWERED_BIT],
    components: [
        MapColorComponent { r: 216, g: 127, b: 51, a: 255 },
        LightEmissionComponent { emission: 15 },
        MineableComponent::hardness(3.0),
    ],
    permutations: [
        const_permutation! {
            condition: |it| it["lit"] == false,
            components: [LightEmissionComponent { emission: 0 }]
        },
    ],
};
