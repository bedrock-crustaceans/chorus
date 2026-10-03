use crate::block_definition::BlockDefinition;
use crate::block_id;
use crate::component::light_emission_component::LightEmissionComponent;
use crate::component::map_color_component::MapColorComponent;
use crate::const_block;
use crate::state::common::PILLAR_AXIS;

pub const VERDANT_FROGLIGHT: BlockDefinition = const_block! {
    identifier: block_id::VERDANT_FROGLIGHT,
    states: [PILLAR_AXIS],
    components: [
        MapColorComponent { r: 127, g: 167, b: 150, a: 255 },
        LightEmissionComponent { emission: 15 },
    ],
    permutations: [],
};
