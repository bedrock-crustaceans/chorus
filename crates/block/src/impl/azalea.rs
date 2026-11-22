use crate::block_definition::BlockDefinition;
use crate::block_id;
use crate::component::collision_box_component::CollisionBoxComponent;
use crate::component::liquid_detection_component::LiquidDetectionComponent;
use crate::component::map_color_component::MapColorComponent;
use crate::component::mineable_component::MineableComponent;
use crate::component::placement_filter_component::PlacementFilterComponent;
use crate::const_block;
use crate::placement;
use glam::Vec3;

pub static AZALEA: BlockDefinition = const_block! {
    identifier: block_id::AZALEA,
    states: [],
    components: [
        MapColorComponent { r: 0, g: 124, b: 0, a: 255 },
        MineableComponent::hardness(0.0),
        LiquidDetectionComponent::BROKEN_BY_WATER_AND_LAVA,
        CollisionBoxComponent::new(Vec3::new(0.0, 0.5, 0.0), Vec3::new(1.0, 0.5, 1.0)),
        PlacementFilterComponent::new(placement::AZALEA),
    ],
    permutations: [],
};
