use crate::block_definition::BlockDefinition;
use crate::block_id;
use crate::component::flammable_component::FlammableComponent;
use crate::component::light_dampening_component::LightDampeningComponent;
use crate::component::mineable_component::MineableComponent;
use crate::component::moveable_component::{MoveableComponent, Movement};
use crate::component::transparent_component::TransparentComponent;
use crate::const_block;
use crate::state::common::{PERSISTENT_BIT, UPDATE_BIT};

pub const RED_POPLAR_LEAVES: BlockDefinition = const_block! {
    identifier: block_id::RED_POPLAR_LEAVES,
    states: [PERSISTENT_BIT, UPDATE_BIT],
    components: [
        TransparentComponent { transparent: true },
        LightDampeningComponent { dampening: 1 },
        FlammableComponent { catch_chance: 30, destroy_chance: 60 },
        MineableComponent::hardness(0.2),
        MoveableComponent { movement: Movement::Break, sticky: false },
    ],
    permutations: [],
};
