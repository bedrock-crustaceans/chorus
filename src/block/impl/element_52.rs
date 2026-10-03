use crate::block::block_definition::BlockDefinition;
use crate::block::block_id;
use crate::block::component::light_dampening_component::LightDampeningComponent;
use crate::block::component::map_color_component::MapColorComponent;
use crate::block::component::mineable_component::MineableComponent;
use crate::block::component::transparent_component::TransparentComponent;
use crate::const_block;

pub const ELEMENT_52: BlockDefinition = const_block! {
    identifier: block_id::ELEMENT_52,
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
