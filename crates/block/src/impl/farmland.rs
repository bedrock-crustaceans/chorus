use crate::block_definition::BlockDefinition;
use crate::block_id;
use crate::component::light_dampening_component::LightDampeningComponent;
use crate::component::map_color_component::MapColorComponent;
use crate::component::mineable_component::MineableComponent;
use crate::component::transparent_component::TransparentComponent;
use crate::const_block;
use crate::state::common::MOISTURIZED_AMOUNT;

pub const FARMLAND: BlockDefinition = const_block! {
    identifier: block_id::FARMLAND,
    states: [MOISTURIZED_AMOUNT],
    components: [
        TransparentComponent { transparent: true },
        MapColorComponent { r: 151, g: 109, b: 77, a: 255 },
        LightDampeningComponent { dampening: 1 },
        MineableComponent::hardness(0.6),
    ],
    permutations: [],
};
