use crate::block::block_definition::BlockDefinition;
use crate::block::component::light_dampening_component::LightDampeningComponent;
use crate::block::component::mineable_component::MineableComponent;
use crate::block::component::solid_component::SolidComponent;
use crate::block::component::transparent_component::TransparentComponent;
use crate::const_block;

pub const GREEN_STAINED_GLASS_PANE: BlockDefinition = const_block! {
    identifier: "minecraft:green_stained_glass_pane",
    states: [],
    components: [
        SolidComponent { solid: false },
        TransparentComponent { transparent: true },
        LightDampeningComponent { dampening: 1 },
        MineableComponent::hardness(0.3),
    ],
    permutations: [],
};
