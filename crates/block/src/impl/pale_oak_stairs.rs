use crate::block_definition::BlockDefinition;
use crate::block_id;
use crate::component::collision_box_component::CollisionBoxComponent;
use crate::component::flammable_component::FlammableComponent;
use crate::component::light_dampening_component::LightDampeningComponent;
use crate::component::map_color_component::MapColorComponent;
use crate::component::mineable_component::MineableComponent;
use crate::component::transparent_component::TransparentComponent;
use crate::state::common::{MINECRAFT_CORNER, UPSIDE_DOWN_BIT, WEIRDO_DIRECTION};
use crate::{const_block, const_permutation};
use glam::Vec3;

pub const PALE_OAK_STAIRS: BlockDefinition = const_block! {
    identifier: block_id::PALE_OAK_STAIRS,
    states: [UPSIDE_DOWN_BIT, WEIRDO_DIRECTION, MINECRAFT_CORNER],
    components: [
        TransparentComponent { transparent: true },
        MapColorComponent { r: 255, g: 252, b: 245, a: 255 },
        LightDampeningComponent { dampening: 1 },
        FlammableComponent { catch_chance: 5, destroy_chance: 20 },
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
