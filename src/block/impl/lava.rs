use crate::block::block_definition::BlockDefinition;
use crate::block::component::collision_box_component::CollisionBoxComponent;
use crate::block::component::internal_friction_component::InternalFrictionComponent;
use crate::block::component::light_dampening_component::LightDampeningComponent;
use crate::block::component::light_emission_component::LightEmissionComponent;
use crate::block::component::map_color_component::MapColorComponent;
use crate::block::component::mineable_component::MineableComponent;
use crate::block::component::moveable_component::{MoveableComponent, Movement};
use crate::block::component::replaceable_component::ReplaceableComponent;
use crate::block::component::solid_component::SolidComponent;
use crate::block::component::transparent_component::TransparentComponent;
use crate::block::state::common::LIQUID_DEPTH;
use crate::{const_block, const_permutation};
use vek::Vec3;

pub const LAVA: BlockDefinition = const_block! {
    identifier: "minecraft:lava",
    states: [LIQUID_DEPTH],
    components: [
        SolidComponent { solid: false },
        TransparentComponent { transparent: true },
        MapColorComponent { r: 255, g: 0, b: 0, a: 255 },
        InternalFrictionComponent { internal_friction: 0.3 },
        LightEmissionComponent { emission: 15 },
        LightDampeningComponent { dampening: 2 },
        ReplaceableComponent { replaceable: true },
        MineableComponent::hardness(100.0),
        MoveableComponent { movement: Movement::Break, sticky: false },
        CollisionBoxComponent { origin: Vec3::new(0.0, 0.0, 0.0), size: Vec3::new(1.0, 0.8888888880610466, 1.0), enabled: false },
    ],
    permutations: [
        const_permutation! {
            condition: |it| it["liquid_depth"] == 1,
            components: [CollisionBoxComponent { origin: Vec3::new(0.0, 0.0, 0.0), size: Vec3::new(1.0, 0.7777777761220932, 1.0), enabled: false }]
        },
        const_permutation! {
            condition: |it| it["liquid_depth"] == 2,
            components: [CollisionBoxComponent { origin: Vec3::new(0.0, 0.0, 0.0), size: Vec3::new(1.0, 0.6666666567325592, 1.0), enabled: false }]
        },
        const_permutation! {
            condition: |it| it["liquid_depth"] == 3,
            components: [CollisionBoxComponent { origin: Vec3::new(0.0, 0.0, 0.0), size: Vec3::new(1.0, 0.5555555522441864, 1.0), enabled: false }]
        },
        const_permutation! {
            condition: |it| it["liquid_depth"] == 4,
            components: [CollisionBoxComponent { origin: Vec3::new(0.0, 0.0, 0.0), size: Vec3::new(1.0, 0.4444444179534912, 1.0), enabled: false }]
        },
        const_permutation! {
            condition: |it| it["liquid_depth"] == 5,
            components: [CollisionBoxComponent { origin: Vec3::new(0.0, 0.0, 0.0), size: Vec3::new(1.0, 0.3333333134651184, 1.0), enabled: false }]
        },
        const_permutation! {
            condition: |it| it["liquid_depth"] == 6,
            components: [CollisionBoxComponent { origin: Vec3::new(0.0, 0.0, 0.0), size: Vec3::new(1.0, 0.2222222089767456, 1.0), enabled: false }]
        },
        const_permutation! {
            condition: |it| it["liquid_depth"] == 7,
            components: [CollisionBoxComponent { origin: Vec3::new(0.0, 0.0, 0.0), size: Vec3::new(1.0, 0.1111111044883728, 1.0), enabled: false }]
        },
    ],
};
