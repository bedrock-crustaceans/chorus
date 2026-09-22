use crate::block::block_definition::BlockDefinition;
use crate::block::component::internal_friction_component::InternalFrictionComponent;
use crate::block::component::light_dampening_component::LightDampeningComponent;
use crate::block::component::map_color_component::MapColorComponent;
use crate::block::component::mineable_component::MineableComponent;
use crate::block::component::moveable_component::{MoveableComponent, Movement};
use crate::block::component::transparent_component::TransparentComponent;
use crate::block::state::common::{AGE_BIT, BAMBOO_LEAF_SIZE, BAMBOO_STALK_THICKNESS};
use crate::const_block;

pub const BAMBOO: BlockDefinition = const_block! {
    identifier: "minecraft:bamboo",
    states: [AGE_BIT, BAMBOO_LEAF_SIZE, BAMBOO_STALK_THICKNESS],
    components: [
        TransparentComponent { transparent: true },
        MapColorComponent { r: 0, g: 124, b: 0, a: 255 },
        InternalFrictionComponent { internal_friction: 0.95 },
        LightDampeningComponent { dampening: 1 },
        MineableComponent::hardness(2.0),
        MoveableComponent { movement: Movement::Break, sticky: false },
    ],
    permutations: [],
};
