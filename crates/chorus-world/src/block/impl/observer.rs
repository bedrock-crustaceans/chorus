use crate::block::block_definition::BlockDefinition;
use crate::block::component::map_color_component::MapColorComponent;
use crate::block::component::mineable_component::MineableComponent;
use crate::block::state::common::{MINECRAFT_FACING_DIRECTION, POWERED_BIT};
use crate::const_block;

pub const OBSERVER: BlockDefinition = const_block! {
    identifier: "minecraft:observer",
    states: [MINECRAFT_FACING_DIRECTION, POWERED_BIT],
    components: [
        MapColorComponent { r: 112, g: 112, b: 112, a: 255 },
        MineableComponent::hardness(3.5),
    ],
    permutations: [],
};
