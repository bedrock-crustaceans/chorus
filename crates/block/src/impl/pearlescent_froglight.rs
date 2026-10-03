use crate::block_definition::BlockDefinition;
use crate::block_id;
use crate::component::light_emission_component::LightEmissionComponent;
use crate::component::map_color_component::MapColorComponent;
use crate::const_block;
use crate::state::common::PILLAR_AXIS;

pub const PEARLESCENT_FROGLIGHT: BlockDefinition = const_block! {
    identifier: block_id::PEARLESCENT_FROGLIGHT,
    states: [PILLAR_AXIS],
    components: [
        MapColorComponent { r: 242, g: 127, b: 165, a: 255 },
        LightEmissionComponent { emission: 15 },
    ],
    permutations: [],
};
