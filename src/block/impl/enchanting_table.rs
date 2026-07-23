use crate::block::block_definition::BlockDefinition;
use crate::block::component::collision_box_component::CollisionBoxComponent;
use crate::block::component::light_dampening_component::LightDampeningComponent;
use crate::block::component::light_emission_component::LightEmissionComponent;
use crate::block::component::map_color_component::MapColorComponent;
use crate::block::component::mineable_component::MineableComponent;
use crate::block::component::transparent_component::TransparentComponent;
use crate::const_block;
use vek::Vec3;

pub const ENCHANTING_TABLE: BlockDefinition = const_block! {
    identifier: "minecraft:enchanting_table",
    states: [],
    components: [
        TransparentComponent { transparent: true },
        MapColorComponent { r: 153, g: 51, b: 51, a: 255 },
        LightEmissionComponent { emission: 7 },
        LightDampeningComponent { dampening: 1 },
        MineableComponent::hardness(5.0),
        CollisionBoxComponent::new(Vec3::new(0.0, 0.0, 0.0), Vec3::new(1.0, 0.75, 1.0)),
    ],
    permutations: [],
};
