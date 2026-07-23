use crate::block::block_definition::BlockDefinition;
use crate::block::component::internal_friction_component::InternalFrictionComponent;
use crate::block::component::light_dampening_component::LightDampeningComponent;
use crate::block::component::map_color_component::MapColorComponent;
use crate::block::component::mineable_component::MineableComponent;
use crate::block::component::moveable_component::{MoveableComponent, Movement};
use crate::block::component::replaceable_component::ReplaceableComponent;
use crate::block::component::solid_component::SolidComponent;
use crate::block::component::transparent_component::TransparentComponent;
use crate::block::state::common::MULTI_FACE_DIRECTION_BITS;
use crate::const_block;

pub const SCULK_VEIN: BlockDefinition = const_block! {
    identifier: "minecraft:sculk_vein",
    states: [MULTI_FACE_DIRECTION_BITS],
    components: [
        SolidComponent { solid: false },
        TransparentComponent { transparent: true },
        MapColorComponent { r: 13, g: 18, b: 23, a: 255 },
        InternalFrictionComponent { internal_friction: 0.95 },
        LightDampeningComponent { dampening: 1 },
        ReplaceableComponent { replaceable: true },
        MineableComponent::hardness(0.2),
        MoveableComponent { movement: Movement::Break, sticky: false },
    ],
    permutations: [],
};
