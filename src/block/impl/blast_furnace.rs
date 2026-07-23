use crate::block::block_definition::BlockDefinition;
use crate::block::component::map_color_component::MapColorComponent;
use crate::block::component::mineable_component::MineableComponent;
use crate::block::state::common::MINECRAFT_CARDINAL_DIRECTION;
use crate::const_block;

pub const BLAST_FURNACE: BlockDefinition = const_block! {
    identifier: "minecraft:blast_furnace",
    states: [MINECRAFT_CARDINAL_DIRECTION],
    components: [
        MapColorComponent { r: 112, g: 112, b: 112, a: 255 },
        MineableComponent::hardness(3.5),
    ],
    permutations: [],
};
