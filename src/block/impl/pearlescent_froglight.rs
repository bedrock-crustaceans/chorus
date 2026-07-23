use crate::block::block_definition::BlockDefinition;
use crate::block::component::light_emission_component::LightEmissionComponent;
use crate::block::component::map_color_component::MapColorComponent;
use crate::block::state::common::PILLAR_AXIS;
use crate::const_block;

pub const PEARLESCENT_FROGLIGHT: BlockDefinition = const_block! {
    identifier: "minecraft:pearlescent_froglight",
    states: [PILLAR_AXIS],
    components: [
        MapColorComponent { r: 242, g: 127, b: 165, a: 255 },
        LightEmissionComponent { emission: 15 },
    ],
    permutations: [],
};
