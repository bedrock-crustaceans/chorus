use crate::block::block_definition::BlockDefinition;
use crate::block::component::map_color_component::MapColorComponent;
use crate::block::component::mineable_component::MineableComponent;
use crate::block::component::moveable_component::{MoveableComponent, Movement};
use crate::const_block;

pub const OBSIDIAN: BlockDefinition = const_block! {
    identifier: "minecraft:obsidian",
    states: [],
    components: [
        MapColorComponent { r: 25, g: 25, b: 25, a: 255 },
        MineableComponent::hardness(35.0),
        MoveableComponent { movement: Movement::None, sticky: false },
    ],
    permutations: [],
};
