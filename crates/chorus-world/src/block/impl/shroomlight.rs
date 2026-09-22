use crate::block::block_definition::BlockDefinition;
use crate::block::component::light_dampening_component::LightDampeningComponent;
use crate::block::component::light_emission_component::LightEmissionComponent;
use crate::block::component::map_color_component::MapColorComponent;
use crate::block::component::mineable_component::MineableComponent;
use crate::block::component::transparent_component::TransparentComponent;
use crate::const_block;

pub const SHROOMLIGHT: BlockDefinition = const_block! {
    identifier: "minecraft:shroomlight",
    states: [],
    components: [
        TransparentComponent { transparent: true },
        MapColorComponent { r: 153, g: 51, b: 51, a: 255 },
        LightEmissionComponent { emission: 15 },
        LightDampeningComponent { dampening: 1 },
        MineableComponent::hardness(1.0),
    ],
    permutations: [],
};
