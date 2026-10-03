use crate::block_definition::BlockDefinition;
use crate::block_id;
use crate::component::flammable_component::FlammableComponent;
use crate::component::map_color_component::MapColorComponent;
use crate::component::mineable_component::MineableComponent;
use crate::const_block;
use crate::state::common::EXPLODE_BIT;

pub const TNT: BlockDefinition = const_block! {
    identifier: block_id::TNT,
    states: [EXPLODE_BIT],
    components: [
        MapColorComponent { r: 255, g: 0, b: 0, a: 255 },
        FlammableComponent { catch_chance: 15, destroy_chance: 100 },
        MineableComponent::hardness(0.0),
    ],
    permutations: [],
};
