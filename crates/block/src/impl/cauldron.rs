use crate::block_definition::BlockDefinition;
use crate::block_id;
use crate::component::collision_box_component::CollisionBoxComponent;
use crate::component::light_dampening_component::LightDampeningComponent;
use crate::component::light_emission_component::LightEmissionComponent;
use crate::component::map_color_component::MapColorComponent;
use crate::component::mineable_component::MineableComponent;
use crate::component::transparent_component::TransparentComponent;
use crate::state::common::{CAULDRON_LIQUID, FILL_LEVEL};
use crate::{const_block, const_permutation};
use glam::Vec3;

pub const CAULDRON: BlockDefinition = const_block! {
    identifier: block_id::CAULDRON,
    states: [CAULDRON_LIQUID, FILL_LEVEL],
    components: [
        TransparentComponent { transparent: true },
        MapColorComponent { r: 112, g: 112, b: 112, a: 255 },
        LightDampeningComponent { dampening: 3 },
        MineableComponent::hardness(2.0),
        CollisionBoxComponent::new(Vec3::new(0.3, 0.3, 0.3), Vec3::new(0.4, 0.4, 0.4)),
    ],
    permutations: [
        const_permutation! {
            condition: |it| it["cauldron_liquid"] == "lava",
            components: [LightEmissionComponent { emission: 15 }]
        },
    ],
};
