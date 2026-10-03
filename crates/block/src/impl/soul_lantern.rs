use crate::block_definition::BlockDefinition;
use crate::block_id;
use crate::component::collision_box_component::CollisionBoxComponent;
use crate::component::light_dampening_component::LightDampeningComponent;
use crate::component::light_emission_component::LightEmissionComponent;
use crate::component::map_color_component::MapColorComponent;
use crate::component::mineable_component::MineableComponent;
use crate::component::moveable_component::{MoveableComponent, Movement};
use crate::component::solid_component::SolidComponent;
use crate::component::transparent_component::TransparentComponent;
use crate::state::common::HANGING;
use crate::{const_block, const_permutation};
use glam::Vec3;

pub const SOUL_LANTERN: BlockDefinition = const_block! {
    identifier: block_id::SOUL_LANTERN,
    states: [HANGING],
    components: [
        SolidComponent { solid: false },
        TransparentComponent { transparent: true },
        MapColorComponent { r: 167, g: 167, b: 167, a: 255 },
        LightEmissionComponent { emission: 10 },
        LightDampeningComponent { dampening: 1 },
        MineableComponent::hardness(3.5),
        MoveableComponent { movement: Movement::Break, sticky: false },
        CollisionBoxComponent::new(Vec3::new(0.3125, 0.0625, 0.3125), Vec3::new(0.375, 0.4375, 0.375)),
    ],
    permutations: [
        const_permutation! {
            condition: |it| it["hanging"] == false,
            components: [CollisionBoxComponent::new(Vec3::new(0.3125, 0.0, 0.3125), Vec3::new(0.375, 0.4375, 0.375))]
        },
    ],
};
