use crate::block_definition::BlockDefinition;
use crate::block_id;
use crate::component::collision_box_component::CollisionBoxComponent;
use crate::component::light_dampening_component::LightDampeningComponent;
use crate::component::map_color_component::MapColorComponent;
use crate::component::mineable_component::MineableComponent;
use crate::component::moveable_component::{MoveableComponent, Movement};
use crate::component::support_component::SupportComponent;
use crate::const_block;
use glam::Vec3;

pub static HONEY_BLOCK: BlockDefinition = const_block! {
    identifier: block_id::HONEY_BLOCK,
    states: [],
    components: [
        MapColorComponent { r: 216, g: 127, b: 51, a: 255 },
        LightDampeningComponent { dampening: 1 },
        MineableComponent::hardness(0.0),
        MoveableComponent { movement: Movement::Both, sticky: true },
        CollisionBoxComponent::new(Vec3::new(0.0625, 0.0, 0.0625), Vec3::new(0.875, 0.9375, 0.875)),
        SupportComponent::TOP,
    ],
    permutations: [],
};
