use crate::block::block_definition::BlockDefinition;
use crate::block::component::light_emission_component::LightEmissionComponent;
use crate::block::component::map_color_component::MapColorComponent;
use crate::block::component::mineable_component::MineableComponent;
use crate::const_block;

pub const MAGMA: BlockDefinition = const_block! {
    identifier: "minecraft:magma",
    states: [],
    components: [
        MapColorComponent { r: 112, g: 2, b: 0, a: 255 },
        LightEmissionComponent { emission: 3 },
        MineableComponent::hardness(0.5),
    ],
    permutations: [],
};
