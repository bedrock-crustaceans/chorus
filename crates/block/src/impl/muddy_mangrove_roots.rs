use crate::block_definition::BlockDefinition;
use crate::block_id;
use crate::component::light_dampening_component::LightDampeningComponent;
use crate::component::map_color_component::MapColorComponent;
use crate::component::mineable_component::MineableComponent;
use crate::component::transparent_component::TransparentComponent;
use crate::const_block;
use crate::state::common::PILLAR_AXIS;

pub const MUDDY_MANGROVE_ROOTS: BlockDefinition = const_block! {
    identifier: block_id::MUDDY_MANGROVE_ROOTS,
    states: [PILLAR_AXIS],
    components: [
        TransparentComponent { transparent: true },
        MapColorComponent { r: 129, g: 86, b: 49, a: 255 },
        LightDampeningComponent { dampening: 1 },
        MineableComponent::hardness(0.7),
    ],
    permutations: [],
};
