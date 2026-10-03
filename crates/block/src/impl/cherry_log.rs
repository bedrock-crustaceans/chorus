use crate::block_definition::BlockDefinition;
use crate::block_id;
use crate::component::flammable_component::FlammableComponent;
use crate::component::map_color_component::MapColorComponent;
use crate::component::mineable_component::MineableComponent;
use crate::state::common::PILLAR_AXIS;
use crate::{const_block, const_permutation};

pub const CHERRY_LOG: BlockDefinition = const_block! {
    identifier: block_id::CHERRY_LOG,
    states: [PILLAR_AXIS],
    components: [
        MapColorComponent { r: 209, g: 177, b: 161, a: 255 },
        FlammableComponent { catch_chance: 5, destroy_chance: 5 },
        MineableComponent::hardness(2.0),
    ],
    permutations: [
        const_permutation! {
            condition: |it| (it["pillar_axis"] == "x") || (it["pillar_axis"] == "z"),
            components: [MapColorComponent { r: 57, g: 41, b: 35, a: 255 }]
        },
    ],
};
