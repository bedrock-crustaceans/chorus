use crate::block_definition::BlockDefinition;
use crate::block_id;
use crate::component::collision_box_component::CollisionBoxComponent;
use crate::component::light_dampening_component::LightDampeningComponent;
use crate::component::liquid_detection_component::LiquidDetectionComponent;
use crate::component::map_color_component::MapColorComponent;
use crate::component::mineable_component::MineableComponent;
use crate::component::moveable_component::{MoveableComponent, Movement};
use crate::component::placement_filter_component::PlacementFilterComponent;
use crate::component::transparent_component::TransparentComponent;
use crate::const_block;
use crate::placement;
use crate::state::common::AGE_16;
use glam::Vec3;

pub static CACTUS: BlockDefinition = const_block! {
    identifier: block_id::CACTUS,
    states: [AGE_16],
    components: [
        TransparentComponent { transparent: true },
        MapColorComponent { r: 0, g: 124, b: 0, a: 255 },
        LightDampeningComponent { dampening: 1 },
        MineableComponent::hardness(0.4),
        MoveableComponent { movement: Movement::Break, sticky: false },
        CollisionBoxComponent::new(Vec3::new(0.0625, 0.0, 0.0625), Vec3::new(0.875, 0.9375, 0.875)),
        LiquidDetectionComponent::CONTAINS_WATER,
        PlacementFilterComponent::new(placement::CACTUS),
    ],
    permutations: [],
};
