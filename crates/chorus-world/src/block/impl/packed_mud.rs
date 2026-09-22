use crate::block::block_definition::BlockDefinition;
use crate::block::component::map_color_component::MapColorComponent;
use crate::block::component::mineable_component::MineableComponent;
use crate::const_block;

pub const PACKED_MUD: BlockDefinition = const_block! {
    identifier: "minecraft:packed_mud",
    states: [],
    components: [
        MapColorComponent { r: 151, g: 109, b: 77, a: 255 },
        MineableComponent::hardness(1.0),
    ],
    permutations: [],
};
