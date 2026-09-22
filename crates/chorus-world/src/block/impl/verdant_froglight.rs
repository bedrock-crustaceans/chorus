use crate::block::block_definition::BlockDefinition;
use crate::block::component::light_emission_component::LightEmissionComponent;
use crate::block::component::map_color_component::MapColorComponent;
use crate::block::state::common::PILLAR_AXIS;
use crate::const_block;

pub const VERDANT_FROGLIGHT: BlockDefinition = const_block! {
    identifier: "minecraft:verdant_froglight",
    states: [PILLAR_AXIS],
    components: [
        MapColorComponent { r: 127, g: 167, b: 150, a: 255 },
        LightEmissionComponent { emission: 15 },
    ],
    permutations: [],
};
