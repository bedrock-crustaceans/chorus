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
use glam::Vec3;

pub static DRAGON_EGG: BlockDefinition = const_block! {
    identifier: block_id::DRAGON_EGG,
    states: [],
    components: [
        TransparentComponent { transparent: true },
        MapColorComponent { r: 25, g: 25, b: 25, a: 255 },
        LightEmissionComponent { emission: 1 },
        LightDampeningComponent { dampening: 1 },
        MineableComponent::hardness(3.0),
        MoveableComponent { movement: Movement::Break, sticky: false },
        CollisionBoxComponent::new(Vec3::new(0.0625, 0.0, 0.0625), Vec3::new(0.875, 1.0, 0.875)),
    ],
    permutations: [],
};
