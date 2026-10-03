use crate::block_definition::BlockDefinition;
use crate::block_id;
use crate::component::collision_box_component::CollisionBoxComponent;
use crate::component::flammable_component::FlammableComponent;
use crate::component::internal_friction_component::InternalFrictionComponent;
use crate::component::light_dampening_component::LightDampeningComponent;
use crate::component::map_color_component::MapColorComponent;
use crate::component::mineable_component::MineableComponent;
use crate::component::moveable_component::{MoveableComponent, Movement};
use crate::component::solid_component::SolidComponent;
use crate::component::transparent_component::TransparentComponent;
use crate::state::common::{BIG_DRIPLEAF_HEAD, BIG_DRIPLEAF_TILT, MINECRAFT_CARDINAL_DIRECTION};
use crate::{const_block, const_permutation};
use glam::Vec3;

pub const BIG_DRIPLEAF: BlockDefinition = const_block! {
    identifier: block_id::BIG_DRIPLEAF,
    states: [BIG_DRIPLEAF_HEAD, BIG_DRIPLEAF_TILT, MINECRAFT_CARDINAL_DIRECTION],
    components: [
        SolidComponent { solid: false },
        TransparentComponent { transparent: true },
        MapColorComponent { r: 0, g: 124, b: 0, a: 255 },
        InternalFrictionComponent { internal_friction: 0.95 },
        LightDampeningComponent { dampening: 1 },
        FlammableComponent { catch_chance: 15, destroy_chance: 100 },
        MineableComponent::hardness(0.0),
        MoveableComponent { movement: Movement::Break, sticky: false },
        CollisionBoxComponent { origin: Vec3::new(0.0, 0.6875, 0.0), size: Vec3::new(1.0, 0.3125, 1.0), enabled: false },
    ],
    permutations: [
        const_permutation! {
            condition: |it| (it["big_dripleaf_tilt"] == "none") || (it["big_dripleaf_tilt"] == "partial_tilt") || (it["big_dripleaf_tilt"] == "unstable"),
            components: [InternalFrictionComponent { internal_friction: 1.0 }]
        },
        const_permutation! {
            condition: |it| it["big_dripleaf_head"] == false,
            components: [CollisionBoxComponent::enabled(false)]
        },
    ],
};
