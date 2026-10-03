use crate::block_definition::BlockDefinition;
use crate::block_id;
use crate::component::light_emission_component::LightEmissionComponent;
use crate::component::map_color_component::MapColorComponent;
use crate::component::mineable_component::MineableComponent;
use crate::component::moveable_component::{MoveableComponent, Movement};
use crate::const_block;
use crate::state::common::BLOOM;

pub const SCULK_CATALYST: BlockDefinition = const_block! {
    identifier: block_id::SCULK_CATALYST,
    states: [BLOOM],
    components: [
        MapColorComponent { r: 13, g: 18, b: 23, a: 255 },
        LightEmissionComponent { emission: 6 },
        MineableComponent::hardness(3.0),
        MoveableComponent { movement: Movement::None, sticky: false },
    ],
    permutations: [],
};
