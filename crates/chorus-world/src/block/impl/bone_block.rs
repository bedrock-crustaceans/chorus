use crate::block::block_definition::BlockDefinition;
use crate::block::component::map_color_component::MapColorComponent;
use crate::block::component::mineable_component::MineableComponent;
use crate::block::state::common::{DEPRECATED, PILLAR_AXIS};
use crate::const_block;

pub const BONE_BLOCK: BlockDefinition = const_block! {
    identifier: "minecraft:bone_block",
    states: [DEPRECATED, PILLAR_AXIS],
    components: [
        MapColorComponent { r: 247, g: 233, b: 163, a: 255 },
        MineableComponent::hardness(2.0),
    ],
    permutations: [],
};
