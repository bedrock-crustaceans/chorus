use crate::block::block_definition::BlockDefinition;
use crate::block::block_id;
use crate::block::component::mineable_component::MineableComponent;
use crate::block::component::moveable_component::{MoveableComponent, Movement};
use crate::const_block;

pub const BARRIER: BlockDefinition = const_block! {
    identifier: block_id::BARRIER,
    states: [],
    components: [
        MineableComponent::hardness(-1.0),
        MoveableComponent { movement: Movement::None, sticky: false },
    ],
    permutations: [],
};
