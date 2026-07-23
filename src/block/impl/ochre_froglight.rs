use crate::block::block_definition::BlockDefinition;
use crate::block::component::light_emission_component::LightEmissionComponent;
use crate::block::component::map_color_component::MapColorComponent;
use crate::block::state::common::PILLAR_AXIS;
use crate::const_block;

pub const OCHRE_FROGLIGHT: BlockDefinition = const_block! {
    identifier: "minecraft:ochre_froglight",
    states: [PILLAR_AXIS],
    components: [
        MapColorComponent { r: 247, g: 233, b: 163, a: 255 },
        LightEmissionComponent { emission: 15 },
    ],
    permutations: [],
};
