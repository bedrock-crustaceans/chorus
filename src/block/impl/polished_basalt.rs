use crate::block::block_definition::BlockDefinition;
use crate::block::component::map_color_component::MapColorComponent;
use crate::block::component::mineable_component::MineableComponent;
use crate::block::state::common::PILLAR_AXIS;
use crate::const_block;

pub const POLISHED_BASALT: BlockDefinition = const_block! {
    identifier: "minecraft:polished_basalt",
    states: [PILLAR_AXIS],
    components: [
        MapColorComponent { r: 25, g: 25, b: 25, a: 255 },
        MineableComponent::hardness(1.25),
    ],
    permutations: [],
};
