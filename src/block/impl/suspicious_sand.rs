use crate::block::block_definition::BlockDefinition;
use crate::block::component::map_color_component::MapColorComponent;
use crate::block::component::mineable_component::MineableComponent;
use crate::block::state::common::{BRUSHED_PROGRESS, HANGING};
use crate::const_block;

pub const SUSPICIOUS_SAND: BlockDefinition = const_block! {
    identifier: "minecraft:suspicious_sand",
    states: [BRUSHED_PROGRESS, HANGING],
    components: [
        MapColorComponent { r: 247, g: 233, b: 163, a: 255 },
        MineableComponent::hardness(0.25),
    ],
    permutations: [],
};
