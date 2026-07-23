use crate::block::block_definition::BlockDefinition;
use crate::block::component::collision_box_component::CollisionBoxComponent;
use crate::block::component::light_dampening_component::LightDampeningComponent;
use crate::block::component::map_color_component::MapColorComponent;
use crate::block::component::mineable_component::MineableComponent;
use crate::block::component::transparent_component::TransparentComponent;
use crate::block::state::common::MINECRAFT_CARDINAL_DIRECTION;
use crate::const_block;
use vek::Vec3;

pub const STONECUTTER_BLOCK: BlockDefinition = const_block! {
    identifier: "minecraft:stonecutter_block",
    states: [MINECRAFT_CARDINAL_DIRECTION],
    components: [
        TransparentComponent { transparent: true },
        MapColorComponent { r: 112, g: 112, b: 112, a: 255 },
        LightDampeningComponent { dampening: 1 },
        MineableComponent::hardness(3.5),
        CollisionBoxComponent::new(Vec3::new(0.0, 0.0, 0.0), Vec3::new(1.0, 0.5625, 1.0)),
    ],
    permutations: [],
};
