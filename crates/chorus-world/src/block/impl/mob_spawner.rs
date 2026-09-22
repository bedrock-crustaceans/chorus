use crate::block::block_definition::BlockDefinition;
use crate::block::component::map_color_component::MapColorComponent;
use crate::block::component::mineable_component::MineableComponent;
use crate::block::component::moveable_component::{MoveableComponent, Movement};
use crate::const_block;

pub const MOB_SPAWNER: BlockDefinition = const_block! {
    identifier: "minecraft:mob_spawner",
    states: [],
    components: [
        MapColorComponent { r: 112, g: 112, b: 112, a: 255 },
        MineableComponent::hardness(5.0),
        MoveableComponent { movement: Movement::None, sticky: false },
    ],
    permutations: [],
};
