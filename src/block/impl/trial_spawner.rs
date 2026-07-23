use crate::block::block_definition::BlockDefinition;
use crate::block::component::map_color_component::MapColorComponent;
use crate::block::state::common::{OMINOUS, TRIAL_SPAWNER_STATE};
use crate::const_block;

pub const TRIAL_SPAWNER: BlockDefinition = const_block! {
    identifier: "minecraft:trial_spawner",
    states: [OMINOUS, TRIAL_SPAWNER_STATE],
    components: [
        MapColorComponent { r: 112, g: 112, b: 112, a: 255 },
    ],
    permutations: [],
};
