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
use crate::state::common::{ATTACHED_BIT, FACING_DIRECTION, GROUND_SIGN_DIRECTION, HANGING};

pub const CHERRY_HANGING_SIGN: BlockDefinition = const_block! {
    identifier: block_id::CHERRY_HANGING_SIGN,
    states: [ATTACHED_BIT, FACING_DIRECTION, GROUND_SIGN_DIRECTION, HANGING],
    components: [
        SolidComponent { solid: false },
        TransparentComponent { transparent: true },
        MapColorComponent { r: 160, g: 77, b: 78, a: 255 },
        LightDampeningComponent { dampening: 1 },
        MineableComponent::hardness(1.0),
        MoveableComponent { movement: Movement::Break, sticky: false },
        CollisionBoxComponent::enabled(false),
    ],
    permutations: [],
};
