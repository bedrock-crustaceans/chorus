use crate::block_definition::BlockDefinition;
use crate::block_id;
use crate::component::map_color_component::MapColorComponent;
use crate::component::mineable_component::MineableComponent;
use crate::component::support_component::SupportComponent;
use crate::const_block;

pub static LIGHT_GRAY_CONCRETE: BlockDefinition = const_block! {
    identifier: block_id::LIGHT_GRAY_CONCRETE,
    states: [],
    components: [
        MapColorComponent { r: 153, g: 153, b: 153, a: 255 },
        MineableComponent::hardness(1.8),
        SupportComponent::ALL,
    ],
    permutations: [],
};
