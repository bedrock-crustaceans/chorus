use crate::block::block_definition::BlockDefinition;
use crate::block::component::light_emission_component::LightEmissionComponent;
use crate::block::component::map_color_component::MapColorComponent;
use crate::block::component::mineable_component::MineableComponent;
use crate::block::component::moveable_component::{MoveableComponent, Movement};
use crate::block::state::common::BLOOM;
use crate::const_block;

pub const SCULK_CATALYST: BlockDefinition = const_block! {
    identifier: "minecraft:sculk_catalyst",
    states: [BLOOM],
    components: [
        MapColorComponent { r: 13, g: 18, b: 23, a: 255 },
        LightEmissionComponent { emission: 6 },
        MineableComponent::hardness(3.0),
        MoveableComponent { movement: Movement::None, sticky: false },
    ],
    permutations: [],
};
