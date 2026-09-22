use crate::block::block_definition::BlockDefinition;
use crate::block::component::collision_box_component::CollisionBoxComponent;
use crate::block::component::flammable_component::FlammableComponent;
use crate::block::component::internal_friction_component::InternalFrictionComponent;
use crate::block::component::light_dampening_component::LightDampeningComponent;
use crate::block::component::map_color_component::MapColorComponent;
use crate::block::component::mineable_component::MineableComponent;
use crate::block::component::moveable_component::{MoveableComponent, Movement};
use crate::block::component::solid_component::SolidComponent;
use crate::block::component::transparent_component::TransparentComponent;
use crate::block::state::common::GROWTH;
use crate::{const_block, const_permutation};

pub const SWEET_BERRY_BUSH: BlockDefinition = const_block! {
    identifier: "minecraft:sweet_berry_bush",
    states: [GROWTH],
    components: [
        SolidComponent { solid: false },
        TransparentComponent { transparent: true },
        MapColorComponent { r: 0, g: 124, b: 0, a: 255 },
        InternalFrictionComponent { internal_friction: 0.95 },
        LightDampeningComponent { dampening: 1 },
        FlammableComponent { catch_chance: 30, destroy_chance: 60 },
        MineableComponent::hardness(0.0),
        MoveableComponent { movement: Movement::Break, sticky: false },
        CollisionBoxComponent::enabled(false),
    ],
    permutations: [
        const_permutation! {
            condition: |it| (it["growth"] == 1) || (it["growth"] == 2) || (it["growth"] == 3) || (it["growth"] == 4) || (it["growth"] == 5) || (it["growth"] == 6) || (it["growth"] == 7),
            components: [MineableComponent::hardness(0.25)]
        },
    ],
};
