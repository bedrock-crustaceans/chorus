use crate::block::block_definition::BlockDefinition;
use crate::block::block_id;
use crate::block::component::light_dampening_component::LightDampeningComponent;
use crate::block::component::map_color_component::MapColorComponent;
use crate::block::component::mineable_component::MineableComponent;
use crate::block::component::solid_component::SolidComponent;
use crate::block::component::transparent_component::TransparentComponent;
use crate::block::state::common::{FACING_DIRECTION, POWERED_BIT};
use crate::const_block;

pub const WAXED_OXIDIZED_LIGHTNING_ROD: BlockDefinition = const_block! {
    identifier: block_id::WAXED_OXIDIZED_LIGHTNING_ROD,
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
