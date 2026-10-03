use crate::block_definition::BlockDefinition;
use crate::block_id;
use crate::component::collision_box_component::CollisionBoxComponent;
use crate::component::light_dampening_component::LightDampeningComponent;
use crate::component::map_color_component::MapColorComponent;
use crate::component::mineable_component::MineableComponent;
use crate::component::moveable_component::{MoveableComponent, Movement};
use crate::component::solid_component::SolidComponent;
use crate::component::transparent_component::TransparentComponent;
use crate::const_block;
use crate::state::common::{CRACKED_STATE, TURTLE_EGG_COUNT};
use glam::Vec3;

pub const TURTLE_EGG: BlockDefinition = const_block! {
    identifier: block_id::TURTLE_EGG,
    states: [CRACKED_STATE, TURTLE_EGG_COUNT],
    components: [
        SolidComponent { solid: false },
        TransparentComponent { transparent: true },
        MapColorComponent { r: 247, g: 233, b: 163, a: 255 },
        LightDampeningComponent { dampening: 1 },
        MineableComponent::hardness(0.5),
        MoveableComponent { movement: Movement::Break, sticky: false },
        CollisionBoxComponent::new(Vec3::new(0.1875, 0.0, 0.1875), Vec3::new(0.5625, 0.4375, 0.5625)),
    ],
    permutations: [],
};
