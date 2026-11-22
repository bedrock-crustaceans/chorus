use crate::block_definition::BlockDefinition;
use crate::block_id;
use crate::component::map_color_component::MapColorComponent;
use crate::component::mineable_component::MineableComponent;
use crate::component::support_component::SupportComponent;
use crate::const_block;

pub static RED_TERRACOTTA: BlockDefinition = const_block! {
    identifier: block_id::RED_TERRACOTTA,
    states: [],
    components: [
        MapColorComponent { r: 142, g: 60, b: 46, a: 255 },
        MineableComponent::hardness(1.25),
        SupportComponent::ALL,
    ],
    permutations: [],
};
