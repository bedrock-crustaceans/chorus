use crate::block::block_definition::BlockDefinition;
use crate::block::component::friction_component::FrictionComponent;
use crate::block::component::light_emission_component::LightEmissionComponent;
use crate::block::component::map_color_component::MapColorComponent;
use crate::block::component::mineable_component::MineableComponent;
use crate::const_block;

pub const BLUE_ICE: BlockDefinition = const_block! {
    identifier: "minecraft:blue_ice",
    states: [],
    components: [
        MapColorComponent { r: 160, g: 160, b: 255, a: 255 },
        FrictionComponent { friction: 0.989 },
        LightEmissionComponent { emission: 4 },
        MineableComponent::hardness(2.8),
    ],
    permutations: [],
};
