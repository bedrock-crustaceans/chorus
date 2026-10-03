use crate::block_definition::BlockDefinition;
use crate::block_id;
use crate::component::light_dampening_component::LightDampeningComponent;
use crate::component::map_color_component::MapColorComponent;
use crate::component::mineable_component::MineableComponent;
use crate::component::solid_component::SolidComponent;
use crate::component::transparent_component::TransparentComponent;
use crate::const_block;
use crate::state::common::GROWING_PLANT_AGE;

pub const CAVE_VINES: BlockDefinition = const_block! {
    identifier: block_id::CAVE_VINES,
    states: [GROWING_PLANT_AGE],
    components: [
        SolidComponent { solid: false },
        TransparentComponent { transparent: true },
        MapColorComponent { r: 0, g: 124, b: 0, a: 255 },
        LightDampeningComponent { dampening: 1 },
        MineableComponent::hardness(0.0),
    ],
    permutations: [],
};
