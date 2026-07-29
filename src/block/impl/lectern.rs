use crate::block::block_definition::BlockDefinition;
use crate::block::component::collision_box_component::CollisionBoxComponent;
use crate::block::component::light_dampening_component::LightDampeningComponent;
use crate::block::component::map_color_component::MapColorComponent;
use crate::block::component::mineable_component::MineableComponent;
use crate::block::component::transparent_component::TransparentComponent;
use crate::block::state::common::{MINECRAFT_CARDINAL_DIRECTION, POWERED_BIT};
use crate::const_block;
use glam::Vec3;

pub const LECTERN: BlockDefinition = const_block! {
    identifier: "minecraft:lectern",
    states: [MINECRAFT_CARDINAL_DIRECTION, POWERED_BIT],
    components: [
        TransparentComponent { transparent: true },
        MapColorComponent { r: 143, g: 119, b: 72, a: 255 },
        LightDampeningComponent { dampening: 1 },
        MineableComponent::hardness(2.5),
        CollisionBoxComponent::new(Vec3::new(0.0, 0.0, 0.0), Vec3::new(1.0, 0.89999, 1.0)),
    ],
    permutations: [],
};
