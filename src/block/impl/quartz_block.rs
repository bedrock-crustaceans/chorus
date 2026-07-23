use crate::block::block_definition::BlockDefinition;
use crate::block::component::map_color_component::MapColorComponent;
use crate::block::component::mineable_component::MineableComponent;
use crate::block::state::common::PILLAR_AXIS;
use crate::const_block;

pub const QUARTZ_BLOCK: BlockDefinition = const_block! {
    identifier: "minecraft:quartz_block",
    states: [PILLAR_AXIS],
    components: [
        MapColorComponent { r: 255, g: 252, b: 245, a: 255 },
        MineableComponent::hardness(0.8),
    ],
    permutations: [],
};
