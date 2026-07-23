use crate::block::block_definition::BlockDefinition;
use crate::block::component::flammable_component::FlammableComponent;
use crate::block::component::light_dampening_component::LightDampeningComponent;
use crate::block::component::map_color_component::MapColorComponent;
use crate::block::component::mineable_component::MineableComponent;
use crate::block::component::moveable_component::{MoveableComponent, Movement};
use crate::block::component::transparent_component::TransparentComponent;
use crate::block::state::common::{PERSISTENT_BIT, UPDATE_BIT};
use crate::const_block;

pub const AZALEA_LEAVES_FLOWERED: BlockDefinition = const_block! {
    identifier: "minecraft:azalea_leaves_flowered",
    states: [PERSISTENT_BIT, UPDATE_BIT],
    components: [
        TransparentComponent { transparent: true },
        MapColorComponent { r: 0, g: 124, b: 0, a: 255 },
        LightDampeningComponent { dampening: 1 },
        FlammableComponent { catch_chance: 30, destroy_chance: 60 },
        MineableComponent::hardness(0.2),
        MoveableComponent { movement: Movement::Break, sticky: false },
    ],
    permutations: [],
};
