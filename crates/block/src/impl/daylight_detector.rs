use crate::block_definition::BlockDefinition;
use crate::block_id;
use crate::component::collision_box_component::CollisionBoxComponent;
use crate::component::light_dampening_component::LightDampeningComponent;
use crate::component::map_color_component::MapColorComponent;
use crate::component::mineable_component::MineableComponent;
use crate::component::solid_component::SolidComponent;
use crate::component::transparent_component::TransparentComponent;
use crate::const_block;
use crate::state::common::REDSTONE_SIGNAL;
use glam::Vec3;

pub const DAYLIGHT_DETECTOR: BlockDefinition = const_block! {
    identifier: block_id::DAYLIGHT_DETECTOR,
    states: [REDSTONE_SIGNAL],
    components: [
        SolidComponent { solid: false },
        TransparentComponent { transparent: true },
        MapColorComponent { r: 143, g: 119, b: 72, a: 255 },
        LightDampeningComponent { dampening: 1 },
        MineableComponent::hardness(0.2),
        CollisionBoxComponent::new(Vec3::new(0.0, 0.0, 0.0), Vec3::new(1.0, 0.625, 1.0)),
    ],
    permutations: [],
};
