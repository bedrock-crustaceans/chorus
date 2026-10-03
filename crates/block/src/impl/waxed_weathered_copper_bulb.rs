use crate::block_definition::BlockDefinition;
use crate::block_id;
use crate::component::light_emission_component::LightEmissionComponent;
use crate::component::map_color_component::MapColorComponent;
use crate::component::mineable_component::MineableComponent;
use crate::state::common::{LIT, POWERED_BIT};
use crate::{const_block, const_permutation};

pub const WAXED_WEATHERED_COPPER_BULB: BlockDefinition = const_block! {
    identifier: block_id::WAXED_WEATHERED_COPPER_BULB,
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
