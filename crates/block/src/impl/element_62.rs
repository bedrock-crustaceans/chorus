use crate::block_definition::BlockDefinition;
use crate::block_id;
use crate::component::light_dampening_component::LightDampeningComponent;
use crate::component::map_color_component::MapColorComponent;
use crate::component::mineable_component::MineableComponent;
use crate::component::transparent_component::TransparentComponent;
use crate::const_block;

pub const ELEMENT_62: BlockDefinition = const_block! {
    identifier: block_id::ELEMENT_62,
    states: [],
    components: [
        TransparentComponent { transparent: true },
        LightDampeningComponent { dampening: 1 },
        MineableComponent::hardness(0.3),
        TransparentComponent { transparent: true },
        MapColorComponent { r: 153, g: 153, b: 153, a: 255 },
        LightDampeningComponent { dampening: 1 },
        MineableComponent::hardness(0.3),
    ],
    permutations: [],
};
