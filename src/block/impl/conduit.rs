use crate::block::block_definition::BlockDefinition;
use crate::block::component::collision_box_component::CollisionBoxComponent;
use crate::block::component::light_dampening_component::LightDampeningComponent;
use crate::block::component::light_emission_component::LightEmissionComponent;
use crate::block::component::map_color_component::MapColorComponent;
use crate::block::component::mineable_component::MineableComponent;
use crate::block::component::transparent_component::TransparentComponent;
use crate::const_block;
use glam::Vec3;

pub const CONDUIT: BlockDefinition = const_block! {
    identifier: "minecraft:conduit",
    states: [],
    components: [
        TransparentComponent { transparent: true },
        MapColorComponent { r: 92, g: 219, b: 213, a: 255 },
        LightEmissionComponent { emission: 15 },
        LightDampeningComponent { dampening: 1 },
        MineableComponent::hardness(3.0),
        CollisionBoxComponent::new(Vec3::new(0.3125, 0.3125, 0.3125), Vec3::new(0.375, 0.375, 0.375)),
    ],
    permutations: [],
};
