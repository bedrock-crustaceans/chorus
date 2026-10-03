use crate::block_definition::BlockDefinition;
use crate::block_id;
use crate::component::light_dampening_component::LightDampeningComponent;
use crate::component::map_color_component::MapColorComponent;
use crate::component::mineable_component::MineableComponent;
use crate::component::transparent_component::TransparentComponent;
use crate::const_block;

pub const ORANGE_STAINED_GLASS: BlockDefinition = const_block! {
    identifier: block_id::ORANGE_STAINED_GLASS,
    states: [],
    components: [
        TransparentComponent { transparent: true },
        MapColorComponent { r: 216, g: 127, b: 51, a: 255 },
        LightDampeningComponent { dampening: 1 },
        MineableComponent::hardness(0.3),
    ],
    permutations: [],
};
