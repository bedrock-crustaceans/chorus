use crate::block_definition::BlockDefinition;
use crate::block_id;
use crate::component::light_dampening_component::LightDampeningComponent;
use crate::component::map_color_component::MapColorComponent;
use crate::component::mineable_component::MineableComponent;
use crate::component::solid_component::SolidComponent;
use crate::component::transparent_component::TransparentComponent;
use crate::const_block;
use crate::state::common::{FACING_DIRECTION, POWERED_BIT};

pub const LIGHTNING_ROD: BlockDefinition = const_block! {
    identifier: block_id::LIGHTNING_ROD,
    states: [FACING_DIRECTION, POWERED_BIT],
    components: [
        SolidComponent { solid: false },
        TransparentComponent { transparent: true },
        MapColorComponent { r: 216, g: 127, b: 51, a: 255 },
        LightDampeningComponent { dampening: 1 },
        MineableComponent::hardness(3.0),
    ],
    permutations: [],
};
