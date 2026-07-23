use crate::block::block_definition::BlockDefinition;
use crate::block::component::light_emission_component::LightEmissionComponent;
use crate::block::component::map_color_component::MapColorComponent;
use crate::block::component::mineable_component::MineableComponent;
use crate::const_block;

pub const LIT_REDSTONE_ORE: BlockDefinition = const_block! {
    identifier: "minecraft:lit_redstone_ore",
    states: [],
    components: [
        MapColorComponent { r: 112, g: 112, b: 112, a: 255 },
        LightEmissionComponent { emission: 9 },
        MineableComponent::hardness(3.0),
    ],
    permutations: [],
};
