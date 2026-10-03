use crate::block_definition::BlockDefinition;
use crate::block_id;
use crate::component::internal_friction_component::InternalFrictionComponent;
use crate::component::light_dampening_component::LightDampeningComponent;
use crate::component::map_color_component::MapColorComponent;
use crate::component::mineable_component::MineableComponent;
use crate::component::moveable_component::{MoveableComponent, Movement};
use crate::component::replaceable_component::ReplaceableComponent;
use crate::component::solid_component::SolidComponent;
use crate::component::transparent_component::TransparentComponent;
use crate::const_block;
use crate::state::common::MULTI_FACE_DIRECTION_BITS;

pub const RESIN_CLUMP: BlockDefinition = const_block! {
    identifier: block_id::RESIN_CLUMP,
    states: [MULTI_FACE_DIRECTION_BITS],
    components: [
        SolidComponent { solid: false },
        TransparentComponent { transparent: true },
        MapColorComponent { r: 159, g: 82, b: 36, a: 255 },
        InternalFrictionComponent { internal_friction: 0.95 },
        LightDampeningComponent { dampening: 1 },
        ReplaceableComponent { replaceable: true },
        MineableComponent::hardness(0.2),
        MoveableComponent { movement: Movement::Break, sticky: false },
    ],
    permutations: [],
};
