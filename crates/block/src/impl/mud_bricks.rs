use crate::block_definition::BlockDefinition;
use crate::block_id;
use crate::component::map_color_component::MapColorComponent;
use crate::component::mineable_component::MineableComponent;
use crate::component::support_component::SupportComponent;
use crate::const_block;

pub static MUD_BRICKS: BlockDefinition = const_block! {
    identifier: block_id::MUD_BRICKS,
    states: [],
    components: [
        MapColorComponent { r: 135, g: 107, b: 98, a: 255 },
        MineableComponent::hardness(3.0),
        SupportComponent::ALL,
    ],
    permutations: [],
};
