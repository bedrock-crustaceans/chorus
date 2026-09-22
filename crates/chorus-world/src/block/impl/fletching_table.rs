use crate::block::block_definition::BlockDefinition;
use crate::block::component::flammable_component::FlammableComponent;
use crate::block::component::map_color_component::MapColorComponent;
use crate::block::component::mineable_component::MineableComponent;
use crate::const_block;

pub const FLETCHING_TABLE: BlockDefinition = const_block! {
    identifier: "minecraft:fletching_table",
    states: [],
    components: [
        MapColorComponent { r: 143, g: 119, b: 72, a: 255 },
        FlammableComponent { catch_chance: 5, destroy_chance: 0 },
        MineableComponent::hardness(2.5),
    ],
    permutations: [],
};
