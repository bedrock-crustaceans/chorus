use crate::block_definition::BlockDefinition;
use crate::block_id;
use crate::component::flammable_component::FlammableComponent;
use crate::component::map_color_component::MapColorComponent;
use crate::component::mineable_component::MineableComponent;
use crate::const_block;

pub const SMITHING_TABLE: BlockDefinition = const_block! {
    identifier: block_id::SMITHING_TABLE,
    states: [],
    components: [
        MapColorComponent { r: 143, g: 119, b: 72, a: 255 },
        FlammableComponent { catch_chance: 5, destroy_chance: 0 },
        MineableComponent::hardness(2.5),
    ],
    permutations: [],
};
