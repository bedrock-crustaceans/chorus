use crate::block_definition::BlockDefinition;
use crate::block_id;
use crate::component::flammable_component::FlammableComponent;
use crate::component::map_color_component::MapColorComponent;
use crate::component::mineable_component::MineableComponent;
use crate::const_block;
use crate::state::common::{MINECRAFT_CARDINAL_DIRECTION, POWERED_BIT, POWERED_SHELF_TYPE};

pub const BAMBOO_SHELF: BlockDefinition = const_block! {
    identifier: block_id::BAMBOO_SHELF,
    states: [MINECRAFT_CARDINAL_DIRECTION, POWERED_BIT, POWERED_SHELF_TYPE],
    components: [
        MapColorComponent { r: 229, g: 229, b: 51, a: 255 },
        FlammableComponent { catch_chance: 5, destroy_chance: 20 },
        MineableComponent::hardness(2.0),
    ],
    permutations: [],
};
