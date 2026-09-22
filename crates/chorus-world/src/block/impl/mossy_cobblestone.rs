use crate::block::block_definition::BlockDefinition;
use crate::block::component::map_color_component::MapColorComponent;
use crate::block::component::mineable_component::MineableComponent;
use crate::const_block;

pub const MOSSY_COBBLESTONE: BlockDefinition = const_block! {
    identifier: "minecraft:mossy_cobblestone",
    states: [],
    components: [
        MapColorComponent { r: 112, g: 112, b: 112, a: 255 },
        MineableComponent::hardness(2.0),
    ],
    permutations: [],
};
