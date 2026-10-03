use crate::block_definition::BlockDefinition;
use crate::block_id;
use crate::component::light_dampening_component::LightDampeningComponent;
use crate::component::map_color_component::MapColorComponent;
use crate::component::mineable_component::MineableComponent;
use crate::component::transparent_component::TransparentComponent;
use crate::const_block;

pub const TINTED_GLASS: BlockDefinition = const_block! {
    identifier: block_id::TINTED_GLASS,
    states: [],
    components: [
        TransparentComponent { transparent: true },
        MapColorComponent { r: 76, g: 76, b: 76, a: 255 },
        LightDampeningComponent { dampening: 1 },
        MineableComponent::hardness(0.3),
    ],
    permutations: [],
};
