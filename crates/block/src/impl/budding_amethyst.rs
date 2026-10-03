use crate::block_definition::BlockDefinition;
use crate::block_id;
use crate::component::map_color_component::MapColorComponent;
use crate::component::mineable_component::MineableComponent;
use crate::component::moveable_component::{MoveableComponent, Movement};
use crate::const_block;

pub const BUDDING_AMETHYST: BlockDefinition = const_block! {
    identifier: block_id::BUDDING_AMETHYST,
    states: [],
    components: [
        MapColorComponent { r: 153, g: 90, b: 205, a: 255 },
        MineableComponent::hardness(1.5),
        MoveableComponent { movement: Movement::Break, sticky: false },
    ],
    permutations: [],
};
