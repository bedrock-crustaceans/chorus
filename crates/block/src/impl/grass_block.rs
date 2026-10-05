use crate::block_definition::BlockDefinition;
use crate::block_id;
use crate::component::loot_component::LootComponent;
use crate::component::mineable_component::MineableComponent;
use crate::const_block;

pub const GRASS_BLOCK: BlockDefinition = const_block! {
    identifier: block_id::GRASS_BLOCK,
    states: [],
    components: [
        MineableComponent::hardness(0.6),
        LootComponent::item(block_id::DIRT),
    ],
    permutations: [],
};
