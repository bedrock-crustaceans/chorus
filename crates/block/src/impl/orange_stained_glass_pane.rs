use crate::block_definition::BlockDefinition;
use crate::block_id;
use crate::component::light_dampening_component::LightDampeningComponent;
use crate::component::liquid_detection_component::LiquidDetectionComponent;
use crate::component::mineable_component::MineableComponent;
use crate::component::transparent_component::TransparentComponent;
use crate::const_block;
use crate::state::common::{MINECRAFT_CONNECTION_EAST, MINECRAFT_CONNECTION_NORTH, MINECRAFT_CONNECTION_SOUTH, MINECRAFT_CONNECTION_WEST};

pub static ORANGE_STAINED_GLASS_PANE: BlockDefinition = const_block! {
    identifier: block_id::ORANGE_STAINED_GLASS_PANE,
    states: [MINECRAFT_CONNECTION_EAST, MINECRAFT_CONNECTION_NORTH, MINECRAFT_CONNECTION_SOUTH, MINECRAFT_CONNECTION_WEST],
    components: [
        TransparentComponent { transparent: true },
        LightDampeningComponent { dampening: 1 },
        MineableComponent::hardness(0.3),
        LiquidDetectionComponent::CONTAINS_WATER,
    ],
    permutations: [],
};
