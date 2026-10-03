use crate::block_definition::BlockDefinition;
use crate::block_id;
use crate::component::collision_box_component::CollisionBoxComponent;
use crate::component::light_dampening_component::LightDampeningComponent;
use crate::component::map_color_component::MapColorComponent;
use crate::component::mineable_component::MineableComponent;
use crate::component::transparent_component::TransparentComponent;
use crate::const_block;
use crate::state::common::MINECRAFT_CARDINAL_DIRECTION;
use glam::Vec3;

pub const EXPOSED_COPPER_CHEST: BlockDefinition = const_block! {
    identifier: block_id::EXPOSED_COPPER_CHEST,
    states: [MINECRAFT_CARDINAL_DIRECTION],
    components: [
        TransparentComponent { transparent: true },
        MapColorComponent { r: 143, g: 119, b: 72, a: 255 },
        LightDampeningComponent { dampening: 1 },
        MineableComponent::hardness(2.5),
        CollisionBoxComponent::new(Vec3::new(0.0625, 0.0, 0.0625), Vec3::new(0.875, 0.9475, 0.875)),
    ],
    permutations: [],
};
