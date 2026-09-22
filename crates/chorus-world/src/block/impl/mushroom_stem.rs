use crate::block::block_definition::BlockDefinition;
use crate::block::component::map_color_component::MapColorComponent;
use crate::block::component::mineable_component::MineableComponent;
use crate::block::state::common::HUGE_MUSHROOM_BITS;
use crate::const_block;

pub const MUSHROOM_STEM: BlockDefinition = const_block! {
    identifier: "minecraft:mushroom_stem",
    states: [HUGE_MUSHROOM_BITS],
    components: [
        MapColorComponent { r: 199, g: 199, b: 199, a: 255 },
        MineableComponent::hardness(0.2),
    ],
    permutations: [],
};
