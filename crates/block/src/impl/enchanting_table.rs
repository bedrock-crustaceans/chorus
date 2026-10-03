use crate::block_definition::BlockDefinition;
use crate::block_id;
use crate::component::collision_box_component::CollisionBoxComponent;
use crate::component::light_dampening_component::LightDampeningComponent;
use crate::component::light_emission_component::LightEmissionComponent;
use crate::component::map_color_component::MapColorComponent;
use crate::component::mineable_component::MineableComponent;
use crate::component::transparent_component::TransparentComponent;
use crate::const_block;
use glam::Vec3;

pub const ENCHANTING_TABLE: BlockDefinition = const_block! {
    identifier: block_id::ENCHANTING_TABLE,
    states: [],
    components: [
        TransparentComponent { transparent: true },
        MapColorComponent { r: 153, g: 51, b: 51, a: 255 },
        LightEmissionComponent { emission: 7 },
        LightDampeningComponent { dampening: 1 },
        MineableComponent::hardness(5.0),
        CollisionBoxComponent::new(Vec3::new(0.0, 0.0, 0.0), Vec3::new(1.0, 0.75, 1.0)),
    ],
    permutations: [],
};
