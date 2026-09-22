use crate::block::block_definition::BlockDefinition;
use crate::block::component::map_color_component::MapColorComponent;
use crate::block::component::mineable_component::MineableComponent;
use crate::block::state::common::COMPOSTER_FILL_LEVEL;
use crate::const_block;

pub const COMPOSTER: BlockDefinition = const_block! {
    identifier: "minecraft:composter",
    states: [COMPOSTER_FILL_LEVEL],
    components: [
        MapColorComponent { r: 143, g: 119, b: 72, a: 255 },
        MineableComponent::hardness(0.6),
    ],
    permutations: [],
};
