use crate::block_definition::BlockDefinition;
use crate::block_id;
use crate::component::collision_box_component::CollisionBoxComponent;
use crate::component::internal_friction_component::InternalFrictionComponent;
use crate::component::light_dampening_component::LightDampeningComponent;
use crate::component::light_emission_component::LightEmissionComponent;
use crate::component::map_color_component::MapColorComponent;
use crate::component::mineable_component::MineableComponent;
use crate::component::moveable_component::{MoveableComponent, Movement};
use crate::component::solid_component::SolidComponent;
use crate::component::transparent_component::TransparentComponent;
use crate::state::common::{CLUSTER_COUNT, DEAD_BIT};
use crate::{const_block, const_permutation};

pub const SEA_PICKLE: BlockDefinition = const_block! {
    identifier: block_id::SEA_PICKLE,
    states: [CLUSTER_COUNT, DEAD_BIT],
    components: [
        SolidComponent { solid: false },
        TransparentComponent { transparent: true },
        MapColorComponent { r: 102, g: 127, b: 51, a: 255 },
        InternalFrictionComponent { internal_friction: 0.95 },
        LightDampeningComponent { dampening: 1 },
        MineableComponent::hardness(0.0),
        MoveableComponent { movement: Movement::Break, sticky: false },
        CollisionBoxComponent::enabled(false),
    ],
    permutations: [
        const_permutation! {
            condition: |it| it["dead_bit"] == false,
            components: [LightEmissionComponent { emission: 6 }]
        },
        const_permutation! {
            condition: |it| it["cluster_count"] == 1 && it["dead_bit"] == false,
            components: [LightEmissionComponent { emission: 9 }]
        },
        const_permutation! {
            condition: |it| it["cluster_count"] == 2 && it["dead_bit"] == false,
            components: [LightEmissionComponent { emission: 12 }]
        },
        const_permutation! {
            condition: |it| it["cluster_count"] == 3 && it["dead_bit"] == false,
            components: [LightEmissionComponent { emission: 15 }]
        },
    ],
};
