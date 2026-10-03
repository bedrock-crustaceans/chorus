use crate::block::block_definition::BlockDefinition;
use crate::block::block_id;
use crate::block::component::flammable_component::FlammableComponent;
use crate::block::component::map_color_component::MapColorComponent;
use crate::block::component::mineable_component::MineableComponent;
use crate::block::state::common::MINECRAFT_VERTICAL_HALF;
use crate::const_block;

pub const BLUE_WOOL_DOUBLE_SLAB: BlockDefinition = const_block! {
    identifier: block_id::BLUE_WOOL_DOUBLE_SLAB,
    states: [MINECRAFT_VERTICAL_HALF],
    components: [
        MapColorComponent { r: 51, g: 76, b: 178, a: 255 },
        FlammableComponent { catch_chance: 30, destroy_chance: 60 },
        MineableComponent::hardness(0.8),
    ],
    permutations: [],
};
