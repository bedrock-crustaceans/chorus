use crate::block::block_definition::BlockDefinition;
use crate::block::component::mineable_component::MineableComponent;
use crate::block::component::moveable_component::{MoveableComponent, Movement};
use crate::const_block;

pub const INVISIBLE_BEDROCK: BlockDefinition = const_block! {
    identifier: "minecraft:invisible_bedrock",
    states: [],
    components: [
        MineableComponent::hardness(-1.0),
        MoveableComponent { movement: Movement::None, sticky: false },
    ],
    permutations: [],
};
