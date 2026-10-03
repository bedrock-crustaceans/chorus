use crate::block_definition::BlockDefinition;
use crate::block_id;
use crate::component::collision_box_component::CollisionBoxComponent;
use crate::component::light_dampening_component::LightDampeningComponent;
use crate::component::map_color_component::MapColorComponent;
use crate::component::mineable_component::MineableComponent;
use crate::component::moveable_component::{MoveableComponent, Movement};
use crate::component::solid_component::SolidComponent;
use crate::component::transparent_component::TransparentComponent;
use crate::const_block;
use crate::state::common::GROUND_SIGN_DIRECTION;

pub const SPRUCE_STANDING_SIGN: BlockDefinition = const_block! {
    identifier: block_id::SPRUCE_STANDING_SIGN,
    states: [GROUND_SIGN_DIRECTION],
    components: [
        SolidComponent { solid: false },
        TransparentComponent { transparent: true },
        MapColorComponent { r: 129, g: 86, b: 49, a: 255 },
        LightDampeningComponent { dampening: 1 },
        MineableComponent::hardness(1.0),
        MoveableComponent { movement: Movement::Break, sticky: false },
        CollisionBoxComponent::enabled(false),
    ],
    permutations: [],
};
