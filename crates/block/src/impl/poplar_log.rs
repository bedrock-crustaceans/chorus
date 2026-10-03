use crate::block_definition::BlockDefinition;
use crate::block_id;
use crate::component::flammable_component::FlammableComponent;
use crate::component::map_color_component::MapColorComponent;
use crate::component::mineable_component::MineableComponent;
use crate::state::common::PILLAR_AXIS;
use crate::{const_block, const_permutation};

pub const POPLAR_LOG: BlockDefinition = const_block! {
    identifier: block_id::POPLAR_LOG,
    states: [PILLAR_AXIS],
    components: [
        MapColorComponent { r: 153, g: 153, b: 153, a: 255 },
        FlammableComponent { catch_chance: 5, destroy_chance: 10 },
        MineableComponent::hardness(2.0),
    ],
    permutations: [
        const_permutation! {
            condition: |it| (it["pillar_axis"] == "x") || (it["pillar_axis"] == "z"),
            components: [MapColorComponent { r: 129, g: 86, b: 49, a: 255 }]
        },
    ],
};
