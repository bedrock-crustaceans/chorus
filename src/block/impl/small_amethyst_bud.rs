use crate::block::block_definition::BlockDefinition;
use crate::block::block_id;
use crate::block::component::light_dampening_component::LightDampeningComponent;
use crate::block::component::light_emission_component::LightEmissionComponent;
use crate::block::component::map_color_component::MapColorComponent;
use crate::block::component::mineable_component::MineableComponent;
use crate::block::component::solid_component::SolidComponent;
use crate::block::component::transparent_component::TransparentComponent;
use crate::block::state::common::MINECRAFT_BLOCK_FACE;
use crate::const_block;

pub const SMALL_AMETHYST_BUD: BlockDefinition = const_block! {
    identifier: block_id::SMALL_AMETHYST_BUD,
    states: [MINECRAFT_BLOCK_FACE],
    components: [
        SolidComponent { solid: false },
        TransparentComponent { transparent: true },
        MapColorComponent { r: 153, g: 90, b: 205, a: 255 },
        LightEmissionComponent { emission: 1 },
        LightDampeningComponent { dampening: 1 },
        MineableComponent::hardness(1.5),
    ],
    permutations: [],
};
