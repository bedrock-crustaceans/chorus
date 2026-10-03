use crate::block_definition::BlockDefinition;
use crate::block_id;
use crate::component::map_color_component::MapColorComponent;
use crate::component::mineable_component::MineableComponent;
use crate::const_block;

pub const MUD: BlockDefinition = const_block! {
    identifier: block_id::MUD,
    states: [],
    components: [
        MapColorComponent { r: 87, g: 92, b: 92, a: 255 },
        MineableComponent::hardness(0.5),
    ],
    permutations: [],
};
