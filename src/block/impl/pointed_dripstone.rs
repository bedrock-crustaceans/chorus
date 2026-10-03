use crate::block::block_definition::BlockDefinition;
use crate::block::block_id;
use crate::block::component::mineable_component::MineableComponent;
use crate::block::component::moveable_component::{MoveableComponent, Movement};
use crate::block::state::common::{DRIPSTONE_THICKNESS, HANGING};
use crate::const_block;

pub const POINTED_DRIPSTONE: BlockDefinition = const_block! {
    identifier: block_id::POINTED_DRIPSTONE,
    states: [DRIPSTONE_THICKNESS, HANGING],
    components: [
        MineableComponent::hardness(1.5),
        MoveableComponent { movement: Movement::None, sticky: false },
    ],
    permutations: [],
};
