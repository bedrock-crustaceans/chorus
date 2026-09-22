use crate::block::block_definition::BlockDefinition;
use crate::block::component::flammable_component::FlammableComponent;
use crate::block::component::map_color_component::MapColorComponent;
use crate::block::component::mineable_component::MineableComponent;
use crate::block::state::common::{DEPRECATED, PILLAR_AXIS};
use crate::const_block;

pub const HAY_BLOCK: BlockDefinition = const_block! {
    identifier: "minecraft:hay_block",
    states: [DEPRECATED, PILLAR_AXIS],
    components: [
        MapColorComponent { r: 229, g: 229, b: 51, a: 255 },
        FlammableComponent { catch_chance: 60, destroy_chance: 20 },
        MineableComponent::hardness(0.5),
    ],
    permutations: [],
};
