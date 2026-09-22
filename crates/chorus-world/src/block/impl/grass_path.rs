use crate::block::block_definition::BlockDefinition;
use crate::block::component::light_dampening_component::LightDampeningComponent;
use crate::block::component::map_color_component::MapColorComponent;
use crate::block::component::mineable_component::MineableComponent;
use crate::block::component::transparent_component::TransparentComponent;
use crate::const_block;

pub const GRASS_PATH: BlockDefinition = const_block! {
    identifier: "minecraft:grass_path",
    states: [],
    components: [
        TransparentComponent { transparent: true },
        MapColorComponent { r: 151, g: 109, b: 77, a: 255 },
        LightDampeningComponent { dampening: 1 },
        MineableComponent::hardness(0.65),
    ],
    permutations: [],
};
