use crate::block_definition::BlockDefinition;
use crate::block_id;
use crate::component::collision_box_component::CollisionBoxComponent;
use crate::component::map_color_component::MapColorComponent;
use crate::component::mineable_component::MineableComponent;
use crate::component::support_component::SupportComponent;
use crate::const_block;
use glam::Vec3;

pub static SOUL_SAND: BlockDefinition = const_block! {
    identifier: block_id::SOUL_SAND,
    states: [],
    components: [
        CollisionBoxComponent::new(Vec3::new(0.0, 0.0, 0.0), Vec3::new(1.0, 0.875, 1.0)),
        MapColorComponent { r: 102, g: 76, b: 51, a: 255 },
        MineableComponent::hardness(0.5),
        SupportComponent::ALL,
    ],
    permutations: [],
};
