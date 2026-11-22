use crate::block_definition::BlockDefinition;
use crate::block_id;
use crate::component::collision_box_component::CollisionBoxComponent;
use crate::component::internal_friction_component::InternalFrictionComponent;
use crate::component::light_dampening_component::LightDampeningComponent;
use crate::component::map_color_component::MapColorComponent;
use crate::component::mineable_component::MineableComponent;
use crate::component::moveable_component::{MoveableComponent, Movement};
use crate::component::placement_filter_component::PlacementFilterComponent;
use crate::component::transparent_component::TransparentComponent;
use crate::const_block;
use crate::placement;
use crate::state::common::{AGE_BIT, BAMBOO_LEAF_SIZE, BAMBOO_STALK_THICKNESS};
use glam::Vec3;

pub static BAMBOO: BlockDefinition = const_block! {
    identifier: block_id::BAMBOO,
    states: [AGE_BIT, BAMBOO_LEAF_SIZE, BAMBOO_STALK_THICKNESS],
    components: [
        TransparentComponent { transparent: true },
        MapColorComponent { r: 0, g: 124, b: 0, a: 255 },
        InternalFrictionComponent { internal_friction: 0.95 },
        LightDampeningComponent { dampening: 1 },
        MineableComponent::hardness(2.0),
        MoveableComponent { movement: Movement::Break, sticky: false },
        CollisionBoxComponent::new(Vec3::new(0.375, 0.0, 0.375), Vec3::new(0.25, 1.0, 0.25)),
        PlacementFilterComponent::new(placement::BAMBOO),
    ],
    permutations: [],
};
