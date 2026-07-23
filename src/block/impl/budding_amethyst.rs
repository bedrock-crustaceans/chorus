use crate::block::block_definition::BlockDefinition;
use crate::block::component::map_color_component::MapColorComponent;
use crate::block::component::mineable_component::MineableComponent;
use crate::block::component::moveable_component::{MoveableComponent, Movement};
use crate::const_block;

pub const BUDDING_AMETHYST: BlockDefinition = const_block! {
    identifier: "minecraft:budding_amethyst",
    states: [],
    components: [
        MapColorComponent { r: 153, g: 90, b: 205, a: 255 },
        MineableComponent::hardness(1.5),
        MoveableComponent { movement: Movement::Break, sticky: false },
    ],
    permutations: [],
};
