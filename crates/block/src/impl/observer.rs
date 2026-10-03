use crate::block_definition::BlockDefinition;
use crate::block_id;
use crate::component::map_color_component::MapColorComponent;
use crate::component::mineable_component::MineableComponent;
use crate::const_block;
use crate::state::common::{MINECRAFT_FACING_DIRECTION, POWERED_BIT};

pub const OBSERVER: BlockDefinition = const_block! {
    identifier: block_id::OBSERVER,
    states: [MINECRAFT_FACING_DIRECTION, POWERED_BIT],
    components: [
        MapColorComponent { r: 112, g: 112, b: 112, a: 255 },
        MineableComponent::hardness(3.5),
    ],
    permutations: [],
};
