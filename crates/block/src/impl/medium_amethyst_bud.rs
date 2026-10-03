use crate::block_definition::BlockDefinition;
use crate::block_id;
use crate::component::light_dampening_component::LightDampeningComponent;
use crate::component::light_emission_component::LightEmissionComponent;
use crate::component::map_color_component::MapColorComponent;
use crate::component::mineable_component::MineableComponent;
use crate::component::solid_component::SolidComponent;
use crate::component::transparent_component::TransparentComponent;
use crate::const_block;
use crate::state::common::MINECRAFT_BLOCK_FACE;

pub const MEDIUM_AMETHYST_BUD: BlockDefinition = const_block! {
    identifier: block_id::MEDIUM_AMETHYST_BUD,
    states: [MINECRAFT_BLOCK_FACE],
    components: [
        SolidComponent { solid: false },
        TransparentComponent { transparent: true },
        MapColorComponent { r: 153, g: 90, b: 205, a: 255 },
        LightEmissionComponent { emission: 2 },
        LightDampeningComponent { dampening: 1 },
        MineableComponent::hardness(1.5),
    ],
    permutations: [],
};
