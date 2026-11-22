use crate::block_definition::BlockDefinition;
use crate::block_id;
use crate::component::collision_box_component::CollisionBoxComponent;
use crate::component::light_dampening_component::LightDampeningComponent;
use crate::component::liquid_detection_component::LiquidDetectionComponent;
use crate::component::map_color_component::MapColorComponent;
use crate::component::mineable_component::MineableComponent;
use crate::component::support_component::SupportComponent;
use crate::component::transparent_component::TransparentComponent;
use crate::const_block;
use crate::state::common::{FACING_DIRECTION, TOGGLE_BIT};
use glam::Vec3;

pub static HOPPER: BlockDefinition = const_block! {
    identifier: block_id::HOPPER,
    states: [FACING_DIRECTION, TOGGLE_BIT],
    components: [
        TransparentComponent { transparent: true },
        MapColorComponent { r: 112, g: 112, b: 112, a: 255 },
        LightDampeningComponent { dampening: 1 },
        MineableComponent::hardness(3.0),
        CollisionBoxComponent::new(Vec3::new(0.0, 0.25, 0.0), Vec3::new(1.0, 0.75, 1.0)),
        LiquidDetectionComponent::CONTAINS_WATER,
        SupportComponent::TOP,
    ],
    permutations: [],
};
