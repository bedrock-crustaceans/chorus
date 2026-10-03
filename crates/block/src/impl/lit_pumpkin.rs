use crate::block_definition::BlockDefinition;
use crate::block_id;
use crate::component::light_emission_component::LightEmissionComponent;
use crate::component::map_color_component::MapColorComponent;
use crate::component::mineable_component::MineableComponent;
use crate::component::moveable_component::{MoveableComponent, Movement};
use crate::const_block;
use crate::state::common::MINECRAFT_CARDINAL_DIRECTION;

pub const LIT_PUMPKIN: BlockDefinition = const_block! {
    identifier: block_id::LIT_PUMPKIN,
    states: [MINECRAFT_CARDINAL_DIRECTION],
    components: [
        MapColorComponent { r: 216, g: 127, b: 51, a: 255 },
        LightEmissionComponent { emission: 15 },
        MineableComponent::hardness(1.0),
        MoveableComponent { movement: Movement::Break, sticky: false },
    ],
    permutations: [],
};
