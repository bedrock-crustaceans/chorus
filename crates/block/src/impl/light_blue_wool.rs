use crate::block_definition::BlockDefinition;
use crate::block_id;
use crate::component::flammable_component::FlammableComponent;
use crate::component::map_color_component::MapColorComponent;
use crate::component::mineable_component::MineableComponent;
use crate::const_block;

pub const LIGHT_BLUE_WOOL: BlockDefinition = const_block! {
    identifier: block_id::LIGHT_BLUE_WOOL,
    states: [],
    components: [
        MapColorComponent { r: 102, g: 153, b: 216, a: 255 },
        FlammableComponent { catch_chance: 30, destroy_chance: 60 },
        MineableComponent::hardness(0.8),
    ],
    permutations: [],
};
