use crate::block_definition::BlockDefinition;
use crate::block_id;
use crate::component::flammable_component::FlammableComponent;
use crate::component::map_color_component::MapColorComponent;
use crate::component::mineable_component::MineableComponent;
use crate::const_block;

pub const BIRCH_PLANKS: BlockDefinition = const_block! {
    identifier: block_id::BIRCH_PLANKS,
    states: [],
    components: [
        MapColorComponent { r: 247, g: 233, b: 163, a: 255 },
        FlammableComponent { catch_chance: 5, destroy_chance: 20 },
        MineableComponent::hardness(2.0),
    ],
    permutations: [],
};
