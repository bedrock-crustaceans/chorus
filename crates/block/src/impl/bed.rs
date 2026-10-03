use crate::block_definition::BlockDefinition;
use crate::block_id;
use crate::component::collision_box_component::CollisionBoxComponent;
use crate::component::light_dampening_component::LightDampeningComponent;
use crate::component::mineable_component::MineableComponent;
use crate::component::moveable_component::{MoveableComponent, Movement};
use crate::component::transparent_component::TransparentComponent;
use crate::const_block;
use crate::state::common::{DIRECTION, HEAD_PIECE_BIT, OCCUPIED_BIT};
use glam::Vec3;

pub const BED: BlockDefinition = const_block! {
    identifier: block_id::BED,
    states: [DIRECTION, HEAD_PIECE_BIT, OCCUPIED_BIT],
    components: [
        TransparentComponent { transparent: true },
        LightDampeningComponent { dampening: 1 },
        MineableComponent::hardness(0.2),
        MoveableComponent { movement: Movement::Break, sticky: false },
        CollisionBoxComponent::new(Vec3::new(0.0, 0.0, 0.0), Vec3::new(1.0, 0.5625, 1.0)),
    ],
    permutations: [],
};
