use crate::block::block_definition::BlockDefinition;
use crate::block::component::map_color_component::MapColorComponent;
use crate::block::component::mineable_component::MineableComponent;
use crate::block::state::common::PILLAR_AXIS;
use crate::const_block;

pub const DEEPSLATE: BlockDefinition = const_block! {
    identifier: "minecraft:deepslate",
    states: [PILLAR_AXIS],
    components: [
        MapColorComponent { r: 100, g: 100, b: 100, a: 255 },
        MineableComponent::hardness(3.0),
    ],
    permutations: [],
};
