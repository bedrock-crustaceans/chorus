use crate::block::block_definition::BlockDefinition;
use crate::block::component::collision_box_component::CollisionBoxComponent;
use crate::block::component::light_dampening_component::LightDampeningComponent;
use crate::block::component::map_color_component::MapColorComponent;
use crate::block::component::mineable_component::MineableComponent;
use crate::block::component::transparent_component::TransparentComponent;
use crate::block::state::common::{UPSIDE_DOWN_BIT, WEIRDO_DIRECTION};
use crate::{const_block, const_permutation};
use vek::Vec3;

pub const DARK_PRISMARINE_STAIRS: BlockDefinition = const_block! {
    identifier: "minecraft:dark_prismarine_stairs",
    states: [UPSIDE_DOWN_BIT, WEIRDO_DIRECTION],
    components: [
        TransparentComponent { transparent: true },
        MapColorComponent { r: 92, g: 219, b: 213, a: 255 },
        LightDampeningComponent { dampening: 1 },
        MineableComponent::hardness(1.5),
        CollisionBoxComponent::new(Vec3::new(0.0, 0.5, 0.0), Vec3::new(1.0, 0.5, 1.0)),
    ],
    permutations: [
        const_permutation! {
            condition: |it| it["upside_down_bit"] == false,
            components: [CollisionBoxComponent::new(Vec3::new(0.0, 0.0, 0.0), Vec3::new(1.0, 0.5, 1.0))]
        },
    ],
};
