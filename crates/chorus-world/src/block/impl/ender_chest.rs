use crate::block::block_definition::BlockDefinition;
use crate::block::component::collision_box_component::CollisionBoxComponent;
use crate::block::component::light_dampening_component::LightDampeningComponent;
use crate::block::component::light_emission_component::LightEmissionComponent;
use crate::block::component::map_color_component::MapColorComponent;
use crate::block::component::mineable_component::MineableComponent;
use crate::block::component::moveable_component::{MoveableComponent, Movement};
use crate::block::component::transparent_component::TransparentComponent;
use crate::block::state::common::MINECRAFT_CARDINAL_DIRECTION;
use crate::const_block;
use glam::Vec3;

pub const ENDER_CHEST: BlockDefinition = const_block! {
    identifier: "minecraft:ender_chest",
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
