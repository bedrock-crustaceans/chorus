use crate::block::block_definition::BlockDefinition;
use crate::block::component::collision_box_component::CollisionBoxComponent;
use crate::block::component::light_dampening_component::LightDampeningComponent;
use crate::block::component::map_color_component::MapColorComponent;
use crate::block::component::mineable_component::MineableComponent;
use crate::block::component::moveable_component::{MoveableComponent, Movement};
use crate::block::component::solid_component::SolidComponent;
use crate::block::component::transparent_component::TransparentComponent;
use crate::block::state::common::{CRACKED_STATE, TURTLE_EGG_COUNT};
use crate::const_block;
use vek::Vec3;

pub const TURTLE_EGG: BlockDefinition = const_block! {
    identifier: "minecraft:turtle_egg",
    states: [CRACKED_STATE, TURTLE_EGG_COUNT],
    components: [
        SolidComponent { solid: false },
        TransparentComponent { transparent: true },
        MapColorComponent { r: 247, g: 233, b: 163, a: 255 },
        LightDampeningComponent { dampening: 1 },
        MineableComponent::hardness(0.5),
        MoveableComponent { movement: Movement::Break, sticky: false },
        CollisionBoxComponent::new(Vec3::new(0.1875, 0.0, 0.1875), Vec3::new(0.5625, 0.4375, 0.5625)),
    ],
    permutations: [],
};
