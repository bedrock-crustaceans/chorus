use crate::block::block_definition::BlockDefinition;
use crate::block::block_id;
use crate::block::component::collision_box_component::CollisionBoxComponent;
use crate::block::component::light_dampening_component::LightDampeningComponent;
use crate::block::component::mineable_component::MineableComponent;
use crate::block::component::moveable_component::{MoveableComponent, Movement};
use crate::block::component::transparent_component::TransparentComponent;
use crate::block::state::common::{HEAD_PIECE_BIT, MINECRAFT_CARDINAL_DIRECTION, OCCUPIED_BIT};
use crate::const_block;
use glam::Vec3;

pub const STRAW_BED: BlockDefinition = const_block! {
    identifier: block_id::STRAW_BED,
    states: [HEAD_PIECE_BIT, MINECRAFT_CARDINAL_DIRECTION, OCCUPIED_BIT],
    components: [
        TransparentComponent { transparent: true },
        LightDampeningComponent { dampening: 1 },
        MineableComponent::hardness(0.2),
        MoveableComponent { movement: Movement::Break, sticky: false },
        CollisionBoxComponent::new(Vec3::new(0.0, 0.0, 0.0), Vec3::new(1.0, 0.5625, 1.0)),
    ],
    permutations: [],
};
