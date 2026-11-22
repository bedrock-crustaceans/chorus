use crate::block_definition::BlockDefinition;
use crate::block_id;
use crate::component::collision_box_component::CollisionBoxComponent;
use crate::component::light_dampening_component::LightDampeningComponent;
use crate::component::map_color_component::MapColorComponent;
use crate::component::transparent_component::TransparentComponent;
use crate::const_block;
use crate::state::common::CRACKED_STATE;
use glam::Vec3;

pub static SNIFFER_EGG: BlockDefinition = const_block! {
    identifier: block_id::SNIFFER_EGG,
    states: [CRACKED_STATE],
    components: [
        TransparentComponent { transparent: true },
        MapColorComponent { r: 153, g: 51, b: 51, a: 255 },
        LightDampeningComponent { dampening: 1 },
        CollisionBoxComponent::new(Vec3::new(0.0625, 0.0, 0.125), Vec3::new(0.875, 0.9375, 0.75)),
    ],
    permutations: [],
};
