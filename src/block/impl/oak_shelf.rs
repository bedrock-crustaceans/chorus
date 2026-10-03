use crate::block::block_definition::BlockDefinition;
use crate::block::block_id;
use crate::block::component::flammable_component::FlammableComponent;
use crate::block::component::map_color_component::MapColorComponent;
use crate::block::component::mineable_component::MineableComponent;
use crate::block::state::common::{MINECRAFT_CARDINAL_DIRECTION, POWERED_BIT, POWERED_SHELF_TYPE};
use crate::const_block;

pub const OAK_SHELF: BlockDefinition = const_block! {
    identifier: block_id::OAK_SHELF,
    states: [MINECRAFT_CARDINAL_DIRECTION, POWERED_BIT, POWERED_SHELF_TYPE],
    components: [
        MapColorComponent { r: 143, g: 119, b: 72, a: 255 },
        FlammableComponent { catch_chance: 5, destroy_chance: 20 },
        MineableComponent::hardness(2.0),
    ],
    permutations: [],
};
