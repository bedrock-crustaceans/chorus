use crate::block_definition::BlockDefinition;
use crate::block_id;
use crate::component::collision_box_component::CollisionBoxComponent;
use crate::component::map_color_component::MapColorComponent;
use crate::component::mineable_component::MineableComponent;
use crate::component::support_component::SupportComponent;
use crate::const_block;
use glam::Vec3;

pub static MUD: BlockDefinition = const_block! {
    identifier: block_id::MUD,
    states: [],
    components: [
        CollisionBoxComponent::new(Vec3::new(0.0, 0.0, 0.0), Vec3::new(1.0, 0.875, 1.0)),
        MapColorComponent { r: 87, g: 92, b: 92, a: 255 },
        MineableComponent::hardness(0.5),
        SupportComponent::ALL,
    ],
    permutations: [],
};
