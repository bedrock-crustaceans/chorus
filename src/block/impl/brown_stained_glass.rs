use crate::block::block_definition::BlockDefinition;
use crate::block::component::light_dampening_component::LightDampeningComponent;
use crate::block::component::map_color_component::MapColorComponent;
use crate::block::component::mineable_component::MineableComponent;
use crate::block::component::transparent_component::TransparentComponent;
use crate::const_block;

pub const BROWN_STAINED_GLASS: BlockDefinition = const_block! {
    identifier: "minecraft:brown_stained_glass",
    states: [],
    components: [
        TransparentComponent { transparent: true },
        MapColorComponent { r: 102, g: 76, b: 51, a: 255 },
        LightDampeningComponent { dampening: 1 },
        MineableComponent::hardness(0.3),
    ],
    permutations: [],
};
