use crate::block::block_definition::BlockDefinition;
use crate::block::component::flammable_component::FlammableComponent;
use crate::block::component::map_color_component::MapColorComponent;
use crate::block::component::mineable_component::MineableComponent;
use crate::block::state::common::{BOOKS_STORED, DIRECTION};
use crate::const_block;

pub const CHISELED_BOOKSHELF: BlockDefinition = const_block! {
    identifier: "minecraft:chiseled_bookshelf",
    states: [BOOKS_STORED, DIRECTION],
    components: [
        MapColorComponent { r: 143, g: 119, b: 72, a: 255 },
        FlammableComponent { catch_chance: 30, destroy_chance: 20 },
        MineableComponent::hardness(1.5),
    ],
    permutations: [],
};
