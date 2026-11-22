use crate::block_definition::BlockDefinition;
use crate::block_id;
use crate::component::liquid_detection_component::LiquidDetectionComponent;
use crate::component::map_color_component::MapColorComponent;
use crate::component::mineable_component::MineableComponent;
use crate::component::support_component::SupportComponent;
use crate::const_block;
use crate::state::common::DIRECTION;

pub static COMPOUND_CREATOR: BlockDefinition = const_block! {
    identifier: block_id::COMPOUND_CREATOR,
    states: [DIRECTION],
    components: [
        MapColorComponent { r: 143, g: 119, b: 72, a: 255 },
        MineableComponent::hardness(2.5),
        LiquidDetectionComponent::BROKEN_BY_WATER,
        SupportComponent::ALL,
    ],
    permutations: [],
};
