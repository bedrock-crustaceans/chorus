use crate::block::block_definition::BlockDefinition;
use crate::block::block_id;
use crate::block::component::map_color_component::MapColorComponent;
use crate::const_block;

pub const NETHERREACTOR: BlockDefinition = const_block! {
    identifier: block_id::NETHERREACTOR,
    states: [],
    components: [
        MapColorComponent { r: 167, g: 167, b: 167, a: 255 },
    ],
    permutations: [],
};
