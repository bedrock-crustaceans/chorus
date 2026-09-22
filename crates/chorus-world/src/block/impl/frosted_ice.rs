use crate::block::block_definition::BlockDefinition;
use crate::block::component::friction_component::FrictionComponent;
use crate::block::component::light_dampening_component::LightDampeningComponent;
use crate::block::component::map_color_component::MapColorComponent;
use crate::block::component::mineable_component::MineableComponent;
use crate::block::component::transparent_component::TransparentComponent;
use crate::block::state::common::AGE_4;
use crate::const_block;

pub const FROSTED_ICE: BlockDefinition = const_block! {
    identifier: "minecraft:frosted_ice",
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
