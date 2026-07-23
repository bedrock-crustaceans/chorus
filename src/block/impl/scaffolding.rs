use crate::block::block_definition::BlockDefinition;
use crate::block::component::collision_box_component::CollisionBoxComponent;
use crate::block::component::flammable_component::FlammableComponent;
use crate::block::component::light_dampening_component::LightDampeningComponent;
use crate::block::component::map_color_component::MapColorComponent;
use crate::block::component::mineable_component::MineableComponent;
use crate::block::component::solid_component::SolidComponent;
use crate::block::component::transparent_component::TransparentComponent;
use crate::block::state::common::{STABILITY, STABILITY_CHECK};
use crate::const_block;
use vek::Vec3;

pub const SCAFFOLDING: BlockDefinition = const_block! {
    identifier: "minecraft:scaffolding",
    states: [STABILITY, STABILITY_CHECK],
    components: [
        SolidComponent { solid: false },
        TransparentComponent { transparent: true },
        MapColorComponent { r: 247, g: 233, b: 163, a: 255 },
        LightDampeningComponent { dampening: 1 },
        FlammableComponent { catch_chance: 60, destroy_chance: 60 },
        MineableComponent::hardness(0.5),
        CollisionBoxComponent::new(Vec3::new(0.0, 0.875, 0.0), Vec3::new(1.0, 0.125, 1.0)),
    ],
    permutations: [],
};
