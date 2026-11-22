use crate::block_definition::BlockDefinition;
use crate::block_id;
use crate::component::collision_box_component::CollisionBoxComponent;
use crate::component::light_dampening_component::LightDampeningComponent;
use crate::component::map_color_component::MapColorComponent;
use crate::component::mineable_component::MineableComponent;
use crate::component::support_component::SupportComponent;
use crate::component::transparent_component::TransparentComponent;
use crate::const_block;
use glam::Vec3;

pub static GRASS_PATH: BlockDefinition = const_block! {
    identifier: block_id::GRASS_PATH,
    states: [],
    components: [
        CollisionBoxComponent::new(Vec3::new(0.0, 0.0, 0.0), Vec3::new(1.0, 0.9375, 1.0)),
        TransparentComponent { transparent: true },
        MapColorComponent { r: 151, g: 109, b: 77, a: 255 },
        LightDampeningComponent { dampening: 1 },
        MineableComponent::hardness(0.65),
        SupportComponent::ALL,
    ],
    permutations: [],
};
