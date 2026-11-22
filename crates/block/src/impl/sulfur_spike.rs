use crate::block_definition::BlockDefinition;
use crate::block_id;
use crate::component::collision_box_component::CollisionBoxComponent;
use crate::component::mineable_component::MineableComponent;
use crate::component::moveable_component::{MoveableComponent, Movement};
use crate::const_block;
use crate::state::common::{DRIPSTONE_THICKNESS, HANGING};
use glam::Vec3;

pub static SULFUR_SPIKE: BlockDefinition = const_block! {
    identifier: block_id::SULFUR_SPIKE,
    states: [DRIPSTONE_THICKNESS, HANGING],
    components: [
        MineableComponent::hardness(1.5),
        MoveableComponent { movement: Movement::None, sticky: false },
        CollisionBoxComponent::new(Vec3::new(0.3125, 0.0, 0.3125), Vec3::new(0.375, 1.0, 0.375)),
    ],
    permutations: [],
};
