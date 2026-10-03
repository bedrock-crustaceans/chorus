use crate::block_definition::BlockDefinition;
use crate::block_id;
use crate::component::map_color_component::MapColorComponent;
use crate::component::mineable_component::MineableComponent;
use crate::const_block;
use crate::state::common::{BRUSHED_PROGRESS, HANGING};

pub const SUSPICIOUS_GRAVEL: BlockDefinition = const_block! {
    identifier: block_id::SUSPICIOUS_GRAVEL,
    states: [BRUSHED_PROGRESS, HANGING],
    components: [
        MapColorComponent { r: 112, g: 112, b: 112, a: 255 },
        MineableComponent::hardness(0.25),
    ],
    permutations: [],
};
