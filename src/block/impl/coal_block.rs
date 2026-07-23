use crate::block::block_definition::BlockDefinition;
use crate::block::component::flammable_component::FlammableComponent;
use crate::block::component::map_color_component::MapColorComponent;
use crate::block::component::mineable_component::MineableComponent;
use crate::const_block;

pub const COAL_BLOCK: BlockDefinition = const_block! {
    identifier: "minecraft:coal_block",
    states: [],
    components: [
        MapColorComponent { r: 25, g: 25, b: 25, a: 255 },
        FlammableComponent { catch_chance: 5, destroy_chance: 5 },
        MineableComponent::hardness(5.0),
    ],
    permutations: [],
};
