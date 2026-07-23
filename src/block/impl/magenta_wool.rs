use crate::block::block_definition::BlockDefinition;
use crate::block::component::flammable_component::FlammableComponent;
use crate::block::component::map_color_component::MapColorComponent;
use crate::block::component::mineable_component::MineableComponent;
use crate::const_block;

pub const MAGENTA_WOOL: BlockDefinition = const_block! {
    identifier: "minecraft:magenta_wool",
    states: [],
    components: [
        MapColorComponent { r: 178, g: 76, b: 216, a: 255 },
        FlammableComponent { catch_chance: 30, destroy_chance: 60 },
        MineableComponent::hardness(0.8),
    ],
    permutations: [],
};
