use crate::block::block_definition::BlockDefinition;
use crate::block::component::map_color_component::MapColorComponent;
use crate::block::component::mineable_component::MineableComponent;
use crate::block::state::common::{FACING_DIRECTION, OPEN_BIT};
use crate::const_block;

pub const BARREL: BlockDefinition = const_block! {
    identifier: "minecraft:barrel",
    states: [FACING_DIRECTION, OPEN_BIT],
    components: [
        MapColorComponent { r: 143, g: 119, b: 72, a: 255 },
        MineableComponent::hardness(2.5),
    ],
    permutations: [],
};
