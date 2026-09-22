use crate::block::block_definition::BlockDefinition;
use crate::block::component::light_dampening_component::LightDampeningComponent;
use crate::block::component::map_color_component::MapColorComponent;
use crate::block::component::mineable_component::MineableComponent;
use crate::block::component::transparent_component::TransparentComponent;
use crate::block::state::common::PILLAR_AXIS;
use crate::const_block;

pub const MUDDY_MANGROVE_ROOTS: BlockDefinition = const_block! {
    identifier: "minecraft:muddy_mangrove_roots",
    states: [PILLAR_AXIS],
    components: [
        TransparentComponent { transparent: true },
        MapColorComponent { r: 129, g: 86, b: 49, a: 255 },
        LightDampeningComponent { dampening: 1 },
        MineableComponent::hardness(0.7),
    ],
    permutations: [],
};
