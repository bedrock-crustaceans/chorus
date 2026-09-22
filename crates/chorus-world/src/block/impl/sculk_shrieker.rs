use crate::block::block_definition::BlockDefinition;
use crate::block::component::light_dampening_component::LightDampeningComponent;
use crate::block::component::map_color_component::MapColorComponent;
use crate::block::component::mineable_component::MineableComponent;
use crate::block::component::moveable_component::{MoveableComponent, Movement};
use crate::block::component::solid_component::SolidComponent;
use crate::block::component::transparent_component::TransparentComponent;
use crate::block::state::common::{ACTIVE, CAN_SUMMON};
use crate::const_block;

pub const SCULK_SHRIEKER: BlockDefinition = const_block! {
    identifier: "minecraft:sculk_shrieker",
    states: [ACTIVE, CAN_SUMMON],
    components: [
        SolidComponent { solid: false },
        TransparentComponent { transparent: true },
        MapColorComponent { r: 13, g: 18, b: 23, a: 255 },
        LightDampeningComponent { dampening: 1 },
        MineableComponent::hardness(3.0),
        MoveableComponent { movement: Movement::None, sticky: false },
    ],
    permutations: [],
};
