use crate::block_definition::BlockDefinition;
use crate::block_id;
use crate::component::light_dampening_component::LightDampeningComponent;
use crate::component::light_emission_component::LightEmissionComponent;
use crate::component::map_color_component::MapColorComponent;
use crate::component::mineable_component::MineableComponent;
use crate::component::moveable_component::{MoveableComponent, Movement};
use crate::component::transparent_component::TransparentComponent;
use crate::const_block;

pub const BEACON: BlockDefinition = const_block! {
    identifier: block_id::BEACON,
    states: [],
    components: [
        TransparentComponent { transparent: true },
        MapColorComponent { r: 92, g: 219, b: 213, a: 255 },
        LightEmissionComponent { emission: 15 },
        LightDampeningComponent { dampening: 1 },
        MineableComponent::hardness(3.0),
        MoveableComponent { movement: Movement::None, sticky: false },
    ],
    permutations: [],
};
