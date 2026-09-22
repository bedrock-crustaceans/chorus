use crate::block::block_definition::BlockDefinition;
use crate::block::component::light_dampening_component::LightDampeningComponent;
use crate::block::component::light_emission_component::LightEmissionComponent;
use crate::block::component::map_color_component::MapColorComponent;
use crate::block::component::mineable_component::MineableComponent;
use crate::block::component::solid_component::SolidComponent;
use crate::block::component::transparent_component::TransparentComponent;
use crate::block::state::common::GROWING_PLANT_AGE;
use crate::const_block;

pub const CAVE_VINES_BODY_WITH_BERRIES: BlockDefinition = const_block! {
    identifier: "minecraft:cave_vines_body_with_berries",
    states: [GROWING_PLANT_AGE],
    components: [
        SolidComponent { solid: false },
        TransparentComponent { transparent: true },
        MapColorComponent { r: 0, g: 124, b: 0, a: 255 },
        LightEmissionComponent { emission: 14 },
        LightDampeningComponent { dampening: 1 },
        MineableComponent::hardness(0.0),
    ],
    permutations: [],
};
