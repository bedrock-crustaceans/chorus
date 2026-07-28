use crate::block::block_definition::BlockDefinition;
use crate::block::component::collision_box_component::CollisionBoxComponent;
use crate::block::component::light_dampening_component::LightDampeningComponent;
use crate::block::component::light_emission_component::LightEmissionComponent;
use crate::block::component::map_color_component::MapColorComponent;
use crate::block::component::mineable_component::MineableComponent;
use crate::block::component::transparent_component::TransparentComponent;
use crate::block::state::common::{CAULDRON_LIQUID, FILL_LEVEL};
use crate::{const_block, const_permutation};
use vek::Vec3;

pub const CAULDRON: BlockDefinition = const_block! {
    identifier: "minecraft:cauldron",
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
