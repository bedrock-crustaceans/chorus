use crate::block::block_definition::BlockDefinition;
use crate::block::component::map_color_component::MapColorComponent;
use crate::block::component::mineable_component::MineableComponent;
use crate::block::state::common::{BRUSHED_PROGRESS, HANGING};
use crate::const_block;

pub const SUSPICIOUS_GRAVEL: BlockDefinition = const_block! {
    identifier: "minecraft:suspicious_gravel",
    states: [BRUSHED_PROGRESS, HANGING],
    components: [
        MapColorComponent { r: 112, g: 112, b: 112, a: 255 },
        MineableComponent::hardness(0.25),
    ],
    permutations: [],
};
