use crate::block_definition::BlockDefinition;
use crate::block_id;
use crate::component::light_emission_component::LightEmissionComponent;
use crate::component::map_color_component::MapColorComponent;
use crate::component::moveable_component::{MoveableComponent, Movement};
use crate::state::common::{CREAKING_HEART_STATE, NATURAL, PILLAR_AXIS};
use crate::{const_block, const_permutation};

pub const CREAKING_HEART: BlockDefinition = const_block! {
    identifier: block_id::CREAKING_HEART,
    states: [CREAKING_HEART_STATE, NATURAL, PILLAR_AXIS],
    components: [
        MapColorComponent { r: 216, g: 127, b: 51, a: 255 },
        MoveableComponent { movement: Movement::None, sticky: false },
    ],
    permutations: [
        const_permutation! {
            condition: |it| (it["creaking_heart_state"] == "dormant") || (it["creaking_heart_state"] == "awake"),
            components: [LightEmissionComponent { emission: 15 }]
        },
    ],
};
