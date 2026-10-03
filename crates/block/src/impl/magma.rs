use crate::block_definition::BlockDefinition;
use crate::block_id;
use crate::component::light_emission_component::LightEmissionComponent;
use crate::component::map_color_component::MapColorComponent;
use crate::component::mineable_component::MineableComponent;
use crate::const_block;

pub const MAGMA: BlockDefinition = const_block! {
    identifier: block_id::MAGMA,
    states: [],
    components: [
        MapColorComponent { r: 112, g: 2, b: 0, a: 255 },
        LightEmissionComponent { emission: 3 },
        MineableComponent::hardness(0.5),
    ],
    permutations: [],
};
