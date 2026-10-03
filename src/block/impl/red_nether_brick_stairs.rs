use crate::block::block_definition::BlockDefinition;
use crate::block::block_id;
use crate::block::component::collision_box_component::CollisionBoxComponent;
use crate::block::component::light_dampening_component::LightDampeningComponent;
use crate::block::component::map_color_component::MapColorComponent;
use crate::block::component::mineable_component::MineableComponent;
use crate::block::component::transparent_component::TransparentComponent;
use crate::block::state::common::{MINECRAFT_CORNER, UPSIDE_DOWN_BIT, WEIRDO_DIRECTION};
use crate::{const_block, const_permutation};
use glam::Vec3;

pub const RED_NETHER_BRICK_STAIRS: BlockDefinition = const_block! {
    identifier: block_id::RED_NETHER_BRICK_STAIRS,
    states: [UPSIDE_DOWN_BIT, WEIRDO_DIRECTION, MINECRAFT_CORNER],
    components: [
        TransparentComponent { transparent: true },
        MapColorComponent { r: 112, g: 2, b: 0, a: 255 },
        LightDampeningComponent { dampening: 1 },
        MineableComponent::hardness(2.0),
        CollisionBoxComponent::new(Vec3::new(0.0, 0.5, 0.0), Vec3::new(1.0, 0.5, 1.0)),
    ],
    permutations: [
        const_permutation! {
            condition: |it| it["upside_down_bit"] == false,
            components: [CollisionBoxComponent::new(Vec3::new(0.0, 0.0, 0.0), Vec3::new(1.0, 0.5, 1.0))]
        },
    ],
};
