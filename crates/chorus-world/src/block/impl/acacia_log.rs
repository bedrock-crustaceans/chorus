use crate::block::block_definition::BlockDefinition;
use crate::block::component::flammable_component::FlammableComponent;
use crate::block::component::map_color_component::MapColorComponent;
use crate::block::component::mineable_component::MineableComponent;
use crate::block::state::common::PILLAR_AXIS;
use crate::{const_block, const_permutation};

pub const ACACIA_LOG: BlockDefinition = const_block! {
    identifier: "minecraft:acacia_log",
    states: [PILLAR_AXIS],
    components: [
        MapColorComponent { r: 216, g: 127, b: 51, a: 255 },
        FlammableComponent { catch_chance: 5, destroy_chance: 10 },
        MineableComponent::hardness(2.0),
    ],
    permutations: [
        const_permutation! {
            condition: |it| (it["pillar_axis"] == "x") || (it["pillar_axis"] == "z"),
            components: [MapColorComponent { r: 112, g: 112, b: 112, a: 255 }]
        },
    ],
};
