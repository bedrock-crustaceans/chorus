use crate::block::block_definition::BlockDefinition;
use crate::block::component::map_color_component::MapColorComponent;
use crate::block::component::mineable_component::MineableComponent;
use crate::block::state::common::PILLAR_AXIS;
use crate::const_block;

pub const INFESTED_DEEPSLATE: BlockDefinition = const_block! {
    identifier: "minecraft:infested_deepslate",
    states: [PILLAR_AXIS],
    components: [
        MapColorComponent { r: 100, g: 100, b: 100, a: 255 },
        MineableComponent::hardness(1.5),
    ],
    permutations: [],
};
