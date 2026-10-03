use crate::block_definition::BlockDefinition;
use crate::block_id;
use crate::component::collision_box_component::CollisionBoxComponent;
use crate::component::light_dampening_component::LightDampeningComponent;
use crate::component::light_emission_component::LightEmissionComponent;
use crate::component::map_color_component::MapColorComponent;
use crate::component::mineable_component::MineableComponent;
use crate::component::solid_component::SolidComponent;
use crate::component::transparent_component::TransparentComponent;
use crate::const_block;
use crate::state::common::{BREWING_STAND_SLOT_A_BIT, BREWING_STAND_SLOT_B_BIT, BREWING_STAND_SLOT_C_BIT};
use glam::Vec3;

pub const BREWING_STAND: BlockDefinition = const_block! {
    identifier: block_id::BREWING_STAND,
    states: [BREWING_STAND_SLOT_A_BIT, BREWING_STAND_SLOT_B_BIT, BREWING_STAND_SLOT_C_BIT],
    components: [
        SolidComponent { solid: false },
        TransparentComponent { transparent: true },
        MapColorComponent { r: 167, g: 167, b: 167, a: 255 },
        LightEmissionComponent { emission: 1 },
        LightDampeningComponent { dampening: 1 },
        MineableComponent::hardness(0.5),
        CollisionBoxComponent::new(Vec3::new(0.4375, 0.0, 0.4375), Vec3::new(0.125, 0.875, 0.125)),
    ],
    permutations: [],
};
