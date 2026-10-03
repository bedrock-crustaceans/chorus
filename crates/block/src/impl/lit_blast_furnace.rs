use crate::block_definition::BlockDefinition;
use crate::block_id;
use crate::component::light_emission_component::LightEmissionComponent;
use crate::component::map_color_component::MapColorComponent;
use crate::component::mineable_component::MineableComponent;
use crate::const_block;
use crate::state::common::MINECRAFT_CARDINAL_DIRECTION;

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
