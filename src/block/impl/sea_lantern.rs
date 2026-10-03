use crate::block::block_definition::BlockDefinition;
use crate::block::block_id;
use crate::block::component::light_dampening_component::LightDampeningComponent;
use crate::block::component::light_emission_component::LightEmissionComponent;
use crate::block::component::map_color_component::MapColorComponent;
use crate::block::component::mineable_component::MineableComponent;
use crate::block::component::transparent_component::TransparentComponent;
use crate::const_block;

pub const SEA_LANTERN: BlockDefinition = const_block! {
    identifier: block_id::SEA_LANTERN,
    states: [],
    components: [
        TransparentComponent { transparent: true },
        MapColorComponent { r: 255, g: 252, b: 245, a: 255 },
        LightEmissionComponent { emission: 15 },
        LightDampeningComponent { dampening: 1 },
        MineableComponent::hardness(0.3),
    ],
    permutations: [],
};
