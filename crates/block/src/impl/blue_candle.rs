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
use crate::state::common::{CANDLES, LIT};
use crate::{const_block, const_permutation};

pub const BLUE_CANDLE: BlockDefinition = const_block! {
    identifier: block_id::BLUE_CANDLE,
    states: [CANDLES, LIT],
    components: [
        SolidComponent { solid: false },
        TransparentComponent { transparent: true },
        MapColorComponent { r: 51, g: 76, b: 178, a: 255 },
        InternalFrictionComponent { internal_friction: 0.95 },
        LightDampeningComponent { dampening: 1 },
        MineableComponent::hardness(0.1),
        MoveableComponent { movement: Movement::Break, sticky: false },
        CollisionBoxComponent::enabled(false),
    ],
    permutations: [
        const_permutation! {
            condition: |it| it["candles"] == 1,
            components: [LightEmissionComponent { emission: 3 }]
        },
        const_permutation! {
            condition: |it| it["candles"] == 2,
            components: [LightEmissionComponent { emission: 6 }]
        },
        const_permutation! {
            condition: |it| it["candles"] == 3,
            components: [LightEmissionComponent { emission: 9 }]
        },
    ],
};
