use crate::block::block_definition::BlockDefinition;
use crate::block::component::collision_box_component::CollisionBoxComponent;
use crate::block::component::light_dampening_component::LightDampeningComponent;
use crate::block::component::map_color_component::MapColorComponent;
use crate::block::component::mineable_component::MineableComponent;
use crate::block::component::solid_component::SolidComponent;
use crate::block::component::transparent_component::TransparentComponent;
use crate::block::state::common::REDSTONE_SIGNAL;
use crate::const_block;
use glam::Vec3;

pub const DAYLIGHT_DETECTOR: BlockDefinition = const_block! {
    identifier: "minecraft:daylight_detector",
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
