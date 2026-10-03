use crate::block_definition::BlockDefinition;
use crate::block_id;
use crate::component::collision_box_component::CollisionBoxComponent;
use crate::component::light_dampening_component::LightDampeningComponent;
use crate::component::mineable_component::MineableComponent;
use crate::component::moveable_component::{MoveableComponent, Movement};
use crate::component::solid_component::SolidComponent;
use crate::component::transparent_component::TransparentComponent;
use crate::const_block;
use crate::state::common::{MINECRAFT_CARDINAL_DIRECTION, OUTPUT_LIT_BIT, OUTPUT_SUBTRACT_BIT};
use glam::Vec3;

pub const UNPOWERED_COMPARATOR: BlockDefinition = const_block! {
    identifier: block_id::UNPOWERED_COMPARATOR,
    states: [MINECRAFT_CARDINAL_DIRECTION, OUTPUT_LIT_BIT, OUTPUT_SUBTRACT_BIT],
    components: [
        SolidComponent { solid: false },
        TransparentComponent { transparent: true },
        LightDampeningComponent { dampening: 1 },
        MineableComponent::hardness(0.0),
        MoveableComponent { movement: Movement::Break, sticky: false },
        CollisionBoxComponent::new(Vec3::new(0.0, 0.0, 0.0), Vec3::new(1.0, 0.125, 1.0)),
    ],
    permutations: [],
};
