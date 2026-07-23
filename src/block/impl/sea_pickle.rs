use crate::block::block_definition::BlockDefinition;
use crate::block::component::collision_box_component::CollisionBoxComponent;
use crate::block::component::internal_friction_component::InternalFrictionComponent;
use crate::block::component::light_dampening_component::LightDampeningComponent;
use crate::block::component::light_emission_component::LightEmissionComponent;
use crate::block::component::map_color_component::MapColorComponent;
use crate::block::component::mineable_component::MineableComponent;
use crate::block::component::moveable_component::{MoveableComponent, Movement};
use crate::block::component::solid_component::SolidComponent;
use crate::block::component::transparent_component::TransparentComponent;
use crate::block::state::common::{CLUSTER_COUNT, DEAD_BIT};
use crate::{const_block, const_permutation};

pub const SEA_PICKLE: BlockDefinition = const_block! {
    identifier: "minecraft:sea_pickle",
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
