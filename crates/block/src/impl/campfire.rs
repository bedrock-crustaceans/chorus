use crate::block_definition::BlockDefinition;
use crate::block_id;
use crate::component::collision_box_component::CollisionBoxComponent;
use crate::component::light_dampening_component::LightDampeningComponent;
use crate::component::light_emission_component::LightEmissionComponent;
use crate::component::liquid_detection_component::LiquidDetectionComponent;
use crate::component::map_color_component::MapColorComponent;
use crate::component::mineable_component::MineableComponent;
use crate::component::moveable_component::{MoveableComponent, Movement};
use crate::component::solid_component::SolidComponent;
use crate::component::transparent_component::TransparentComponent;
use crate::state::common::{EXTINGUISHED, MINECRAFT_CARDINAL_DIRECTION};
use crate::{const_block, const_permutation};
use glam::Vec3;

pub static CAMPFIRE: BlockDefinition = const_block! {
    identifier: block_id::CAMPFIRE,
    states: [EXTINGUISHED, MINECRAFT_CARDINAL_DIRECTION],
    components: [
        TransparentComponent { transparent: true },
        MapColorComponent { r: 129, g: 86, b: 49, a: 255 },
        LightDampeningComponent { dampening: 1 },
        MineableComponent::hardness(5.0),
        MoveableComponent { movement: Movement::Break, sticky: false },
        CollisionBoxComponent::new(Vec3::new(0.0, 0.0, 0.0), Vec3::new(1.0, 0.4375, 1.0)),
        LiquidDetectionComponent::CONTAINS_WATER,
        SolidComponent { solid: false },
    ],
    permutations: [
        const_permutation! {
            condition: |it| it["extinguished"] == false,
            components: [LightEmissionComponent { emission: 15 }]
        },
    ],
};
