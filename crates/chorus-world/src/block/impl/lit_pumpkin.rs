use crate::block::block_definition::BlockDefinition;
use crate::block::component::light_emission_component::LightEmissionComponent;
use crate::block::component::map_color_component::MapColorComponent;
use crate::block::component::mineable_component::MineableComponent;
use crate::block::component::moveable_component::{MoveableComponent, Movement};
use crate::block::state::common::MINECRAFT_CARDINAL_DIRECTION;
use crate::const_block;

pub const LIT_PUMPKIN: BlockDefinition = const_block! {
    identifier: "minecraft:lit_pumpkin",
    states: [MINECRAFT_CARDINAL_DIRECTION],
    components: [
        MapColorComponent { r: 216, g: 127, b: 51, a: 255 },
        LightEmissionComponent { emission: 15 },
        MineableComponent::hardness(1.0),
        MoveableComponent { movement: Movement::Break, sticky: false },
    ],
    permutations: [],
};
