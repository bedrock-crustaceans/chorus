use crate::block::block_definition::BlockDefinition;
use crate::block::component::flammable_component::FlammableComponent;
use crate::block::component::map_color_component::MapColorComponent;
use crate::block::component::mineable_component::MineableComponent;
use crate::const_block;

pub const DARK_OAK_PLANKS: BlockDefinition = const_block! {
    identifier: "minecraft:dark_oak_planks",
    states: [],
    components: [
        MapColorComponent { r: 102, g: 76, b: 51, a: 255 },
        FlammableComponent { catch_chance: 5, destroy_chance: 20 },
        MineableComponent::hardness(2.0),
    ],
    permutations: [],
};
