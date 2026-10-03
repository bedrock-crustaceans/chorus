use crate::block_definition::BlockDefinition;
use crate::block_id;
use crate::component::internal_friction_component::InternalFrictionComponent;
use crate::component::light_dampening_component::LightDampeningComponent;
use crate::component::map_color_component::MapColorComponent;
use crate::component::mineable_component::MineableComponent;
use crate::component::replaceable_component::ReplaceableComponent;
use crate::component::solid_component::SolidComponent;
use crate::component::transparent_component::TransparentComponent;
use crate::const_block;
use crate::state::common::DRAG_DOWN;

pub const BUBBLE_COLUMN: BlockDefinition = const_block! {
    identifier: block_id::BUBBLE_COLUMN,
    states: [DRAG_DOWN],
    components: [
        SolidComponent { solid: false },
        TransparentComponent { transparent: true },
        MapColorComponent { r: 64, g: 64, b: 255, a: 255 },
        InternalFrictionComponent { internal_friction: 0.95 },
        LightDampeningComponent { dampening: 1 },
        ReplaceableComponent { replaceable: true },
        MineableComponent::hardness(100.0),
    ],
    permutations: [],
};
