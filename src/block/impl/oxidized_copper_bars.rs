use crate::block::block_definition::BlockDefinition;
use crate::block::block_id;
use crate::block::component::light_dampening_component::LightDampeningComponent;
use crate::block::component::mineable_component::MineableComponent;
use crate::block::component::solid_component::SolidComponent;
use crate::block::component::transparent_component::TransparentComponent;
use crate::block::state::common::{MINECRAFT_CONNECTION_EAST, MINECRAFT_CONNECTION_NORTH, MINECRAFT_CONNECTION_SOUTH, MINECRAFT_CONNECTION_WEST};
use crate::const_block;

pub const OXIDIZED_COPPER_BARS: BlockDefinition = const_block! {
    identifier: block_id::OXIDIZED_COPPER_BARS,
    states: [MINECRAFT_CONNECTION_EAST, MINECRAFT_CONNECTION_NORTH, MINECRAFT_CONNECTION_SOUTH, MINECRAFT_CONNECTION_WEST],
    components: [
        SolidComponent { solid: false },
        TransparentComponent { transparent: true },
        LightDampeningComponent { dampening: 1 },
        MineableComponent::hardness(5.0),
    ],
    permutations: [],
};
