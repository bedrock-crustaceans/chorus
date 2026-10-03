use crate::block_definition::BlockDefinition;
use crate::block_id;
use crate::component::friction_component::FrictionComponent;
use crate::component::light_dampening_component::LightDampeningComponent;
use crate::component::map_color_component::MapColorComponent;
use crate::component::mineable_component::MineableComponent;
use crate::component::transparent_component::TransparentComponent;
use crate::const_block;
use crate::state::common::AGE_4;

pub const FROSTED_ICE: BlockDefinition = const_block! {
    identifier: block_id::FROSTED_ICE,
    states: [AGE_4],
    components: [
        TransparentComponent { transparent: true },
        MapColorComponent { r: 160, g: 160, b: 255, a: 255 },
        FrictionComponent { friction: 0.98 },
        LightDampeningComponent { dampening: 1 },
        MineableComponent::hardness(0.5),
    ],
    permutations: [],
};
