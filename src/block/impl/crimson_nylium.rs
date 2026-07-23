use crate::block::block_definition::BlockDefinition;
use crate::block::component::map_color_component::MapColorComponent;
use crate::block::component::mineable_component::MineableComponent;
use crate::const_block;

pub const CRIMSON_NYLIUM: BlockDefinition = const_block! {
    identifier: "minecraft:crimson_nylium",
    states: [],
    components: [
        MapColorComponent { r: 189, g: 48, b: 49, a: 255 },
        MineableComponent::hardness(0.4),
    ],
    permutations: [],
};
