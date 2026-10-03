use crate::block_definition::BlockDefinition;
use crate::block_id;
use crate::component::collision_box_component::CollisionBoxComponent;
use crate::component::light_dampening_component::LightDampeningComponent;
use crate::component::light_emission_component::LightEmissionComponent;
use crate::component::map_color_component::MapColorComponent;
use crate::component::mineable_component::MineableComponent;
use crate::component::moveable_component::{MoveableComponent, Movement};
use crate::component::transparent_component::TransparentComponent;
use crate::const_block;
use crate::state::common::MINECRAFT_CARDINAL_DIRECTION;
use glam::Vec3;

pub const ENDER_CHEST: BlockDefinition = const_block! {
    identifier: block_id::ENDER_CHEST,
    states: [MINECRAFT_CARDINAL_DIRECTION],
    components: [
        TransparentComponent { transparent: true },
        MapColorComponent { r: 112, g: 112, b: 112, a: 255 },
        LightEmissionComponent { emission: 7 },
        LightDampeningComponent { dampening: 1 },
        MineableComponent::hardness(22.5),
        MoveableComponent { movement: Movement::None, sticky: false },
        CollisionBoxComponent::new(Vec3::new(0.0625, 0.0, 0.0625), Vec3::new(0.875, 0.9475, 0.875)),
    ],
    permutations: [],
};
