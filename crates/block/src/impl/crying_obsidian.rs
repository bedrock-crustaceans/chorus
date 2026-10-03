use crate::block_definition::BlockDefinition;
use crate::block_id;
use crate::component::light_emission_component::LightEmissionComponent;
use crate::component::map_color_component::MapColorComponent;
use crate::component::mineable_component::MineableComponent;
use crate::component::moveable_component::{MoveableComponent, Movement};
use crate::const_block;

pub const CRYING_OBSIDIAN: BlockDefinition = const_block! {
    identifier: block_id::CRYING_OBSIDIAN,
    states: [],
    components: [
        MapColorComponent { r: 25, g: 25, b: 25, a: 255 },
        LightEmissionComponent { emission: 10 },
        MineableComponent::hardness(50.0),
        MoveableComponent { movement: Movement::None, sticky: false },
    ],
    permutations: [],
};
