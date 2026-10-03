use crate::block_definition::BlockDefinition;
use crate::block_id;
use crate::component::mineable_component::MineableComponent;
use crate::component::moveable_component::{MoveableComponent, Movement};
use crate::const_block;
use crate::state::common::{DRIPSTONE_THICKNESS, HANGING};

pub const POINTED_DRIPSTONE: BlockDefinition = const_block! {
    identifier: block_id::POINTED_DRIPSTONE,
    states: [DRIPSTONE_THICKNESS, HANGING],
    components: [
        MineableComponent::hardness(1.5),
        MoveableComponent { movement: Movement::None, sticky: false },
    ],
    permutations: [],
};
