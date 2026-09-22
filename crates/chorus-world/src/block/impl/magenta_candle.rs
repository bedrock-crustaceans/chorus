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
use crate::block::state::common::{CANDLES, LIT};
use crate::{const_block, const_permutation};

pub const MAGENTA_CANDLE: BlockDefinition = const_block! {
    identifier: "minecraft:magenta_candle",
    states: [CANDLES, LIT],
    components: [
        SolidComponent { solid: false },
        TransparentComponent { transparent: true },
        MapColorComponent { r: 178, g: 76, b: 216, a: 255 },
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
