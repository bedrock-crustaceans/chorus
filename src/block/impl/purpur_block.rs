use crate::block::block_definition::BlockDefinition;
use crate::block::component::map_color_component::MapColorComponent;
use crate::block::component::mineable_component::MineableComponent;
use crate::block::state::common::PILLAR_AXIS;
use crate::const_block;

pub const PURPUR_BLOCK: BlockDefinition = const_block! {
    identifier: "minecraft:purpur_block",
    states: [PILLAR_AXIS],
    components: [
        MapColorComponent { r: 178, g: 76, b: 216, a: 255 },
        MineableComponent::hardness(1.5),
    ],
    permutations: [],
};
