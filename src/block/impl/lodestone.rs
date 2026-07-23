use crate::block::block_definition::BlockDefinition;
use crate::block::component::map_color_component::MapColorComponent;
use crate::block::component::mineable_component::MineableComponent;
use crate::block::component::moveable_component::{MoveableComponent, Movement};
use crate::const_block;

pub const LODESTONE: BlockDefinition = const_block! {
    identifier: "minecraft:lodestone",
    states: [],
    components: [
        MapColorComponent { r: 255, g: 255, b: 255, a: 255 },
        MineableComponent::hardness(2.0),
        MoveableComponent { movement: Movement::None, sticky: false },
    ],
    permutations: [],
};
