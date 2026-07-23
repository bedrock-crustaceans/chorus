use crate::block::block_definition::BlockDefinition;
use crate::block::component::collision_box_component::CollisionBoxComponent;
use crate::block::component::light_dampening_component::LightDampeningComponent;
use crate::block::component::light_emission_component::LightEmissionComponent;
use crate::block::component::map_color_component::MapColorComponent;
use crate::block::component::mineable_component::MineableComponent;
use crate::block::component::moveable_component::{MoveableComponent, Movement};
use crate::block::component::transparent_component::TransparentComponent;
use crate::block::state::common::{END_PORTAL_EYE_BIT, MINECRAFT_CARDINAL_DIRECTION};
use crate::{const_block, const_permutation};
use vek::Vec3;

pub const END_PORTAL_FRAME: BlockDefinition = const_block! {
    identifier: "minecraft:end_portal_frame",
    states: [END_PORTAL_EYE_BIT, MINECRAFT_CARDINAL_DIRECTION],
    components: [
        TransparentComponent { transparent: true },
        MapColorComponent { r: 102, g: 127, b: 51, a: 255 },
        LightEmissionComponent { emission: 1 },
        LightDampeningComponent { dampening: 1 },
        MineableComponent::hardness(-1.0),
        MoveableComponent { movement: Movement::None, sticky: false },
    ],
    permutations: [
        const_permutation! {
            condition: |it| it["end_portal_eye_bit"] == false,
            components: [CollisionBoxComponent::new(Vec3::new(0.0, 0.0, 0.0), Vec3::new(1.0, 0.8125, 1.0))]
        },
    ],
};
