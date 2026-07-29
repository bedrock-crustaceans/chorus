use crate::block::block_definition::BlockDefinition;
use crate::block::component::collision_box_component::CollisionBoxComponent;
use crate::block::component::flammable_component::FlammableComponent;
use crate::block::component::light_dampening_component::LightDampeningComponent;
use crate::block::component::map_color_component::MapColorComponent;
use crate::block::component::mineable_component::MineableComponent;
use crate::block::component::transparent_component::TransparentComponent;
use crate::const_block;
use glam::Vec3;

pub const BAMBOO_FENCE: BlockDefinition = const_block! {
    identifier: "minecraft:bamboo_fence",
    states: [],
    components: [
        TransparentComponent { transparent: true },
        MapColorComponent { r: 229, g: 229, b: 51, a: 255 },
        LightDampeningComponent { dampening: 1 },
        FlammableComponent { catch_chance: 5, destroy_chance: 20 },
        MineableComponent::hardness(2.0),
        CollisionBoxComponent::new(Vec3::new(0.0, 0.0, 0.0), Vec3::new(1.0, 1.5, 1.0)),
    ],
    permutations: [],
};
