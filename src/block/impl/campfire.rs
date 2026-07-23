use crate::block::block_definition::BlockDefinition;
use crate::block::component::light_dampening_component::LightDampeningComponent;
use crate::block::component::light_emission_component::LightEmissionComponent;
use crate::block::component::map_color_component::MapColorComponent;
use crate::block::component::mineable_component::MineableComponent;
use crate::block::component::moveable_component::{MoveableComponent, Movement};
use crate::block::component::transparent_component::TransparentComponent;
use crate::block::state::common::{EXTINGUISHED, MINECRAFT_CARDINAL_DIRECTION};
use crate::{const_block, const_permutation};

pub const CAMPFIRE: BlockDefinition = const_block! {
    identifier: "minecraft:campfire",
    states: [EXTINGUISHED, MINECRAFT_CARDINAL_DIRECTION],
    components: [
        TransparentComponent { transparent: true },
        MapColorComponent { r: 129, g: 86, b: 49, a: 255 },
        LightDampeningComponent { dampening: 1 },
        MineableComponent::hardness(5.0),
        MoveableComponent { movement: Movement::Break, sticky: false },
    ],
    permutations: [
        const_permutation! {
            condition: |it| it["extinguished"] == false,
            components: [LightEmissionComponent { emission: 15 }]
        },
    ],
};
