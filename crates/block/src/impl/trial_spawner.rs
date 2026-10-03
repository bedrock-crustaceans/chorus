use crate::block_definition::BlockDefinition;
use crate::block_id;
use crate::component::map_color_component::MapColorComponent;
use crate::const_block;
use crate::state::common::{OMINOUS, TRIAL_SPAWNER_STATE};

pub const TRIAL_SPAWNER: BlockDefinition = const_block! {
    identifier: block_id::TRIAL_SPAWNER,
    states: [OMINOUS, TRIAL_SPAWNER_STATE],
    components: [
        MapColorComponent { r: 112, g: 112, b: 112, a: 255 },
    ],
    permutations: [],
};
