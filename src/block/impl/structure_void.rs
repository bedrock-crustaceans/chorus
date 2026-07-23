use crate::block::block_definition::BlockDefinition;
use crate::block::component::internal_friction_component::InternalFrictionComponent;
use crate::block::component::mineable_component::MineableComponent;
use crate::block::component::moveable_component::{MoveableComponent, Movement};
use crate::const_block;

pub const STRUCTURE_VOID: BlockDefinition = const_block! {
    identifier: "minecraft:structure_void",
    states: [],
    components: [
        InternalFrictionComponent { internal_friction: 0.95 },
        MineableComponent::hardness(0.0),
        MoveableComponent { movement: Movement::None, sticky: false },
    ],
    permutations: [],
};
