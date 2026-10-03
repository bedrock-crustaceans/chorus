use crate::block::block_definition::BlockDefinition;
use crate::block::block_id;
use crate::block::component::light_emission_component::LightEmissionComponent;
use crate::block::component::map_color_component::MapColorComponent;
use crate::block::component::mineable_component::MineableComponent;
use crate::block::state::common::MINECRAFT_CARDINAL_DIRECTION;
use crate::const_block;

pub const LIT_BLAST_FURNACE: BlockDefinition = const_block! {
    identifier: block_id::LIT_BLAST_FURNACE,
    states: [MINECRAFT_CARDINAL_DIRECTION],
    components: [
        MapColorComponent { r: 112, g: 112, b: 112, a: 255 },
        LightEmissionComponent { emission: 13 },
        MineableComponent::hardness(3.5),
    ],
    permutations: [],
};
