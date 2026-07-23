use crate::block::block_definition::BlockDefinition;
use crate::block::component::flammable_component::FlammableComponent;
use crate::block::component::map_color_component::MapColorComponent;
use crate::block::component::mineable_component::MineableComponent;
use crate::block::state::common::EXPLODE_BIT;
use crate::const_block;

pub const TNT: BlockDefinition = const_block! {
    identifier: "minecraft:tnt",
    states: [EXPLODE_BIT],
    components: [
        MapColorComponent { r: 255, g: 0, b: 0, a: 255 },
        FlammableComponent { catch_chance: 15, destroy_chance: 100 },
        MineableComponent::hardness(0.0),
    ],
    permutations: [],
};
