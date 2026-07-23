use crate::block::block_definition::BlockDefinition;
use crate::block::component::light_emission_component::LightEmissionComponent;
use crate::block::component::map_color_component::MapColorComponent;
use crate::block::component::mineable_component::MineableComponent;
use crate::block::component::moveable_component::{MoveableComponent, Movement};
use crate::block::state::common::RESPAWN_ANCHOR_CHARGE;
use crate::{const_block, const_permutation};

pub const RESPAWN_ANCHOR: BlockDefinition = const_block! {
    identifier: "minecraft:respawn_anchor",
    states: [RESPAWN_ANCHOR_CHARGE],
    components: [
        MapColorComponent { r: 25, g: 25, b: 25, a: 255 },
        MineableComponent::hardness(50.0),
        MoveableComponent { movement: Movement::None, sticky: false },
    ],
    permutations: [
        const_permutation! {
            condition: |it| it["respawn_anchor_charge"] == 1,
            components: [LightEmissionComponent { emission: 3 }]
        },
        const_permutation! {
            condition: |it| it["respawn_anchor_charge"] == 2,
            components: [LightEmissionComponent { emission: 7 }]
        },
        const_permutation! {
            condition: |it| (it["respawn_anchor_charge"] == 3) || (it["respawn_anchor_charge"] == 4),
            components: [LightEmissionComponent { emission: 15 }]
        },
    ],
};
