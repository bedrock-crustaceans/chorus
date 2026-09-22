use crate::block::block_definition::BlockDefinition;
use crate::block::component::map_color_component::MapColorComponent;
use crate::block::component::mineable_component::MineableComponent;
use crate::block::component::moveable_component::{MoveableComponent, Movement};
use crate::const_block;

pub const MELON_BLOCK: BlockDefinition = const_block! {
    identifier: "minecraft:melon_block",
    states: [],
    components: [
        MapColorComponent { r: 127, g: 204, b: 25, a: 255 },
        MineableComponent::hardness(1.0),
        MoveableComponent { movement: Movement::Break, sticky: false },
    ],
    permutations: [],
};
