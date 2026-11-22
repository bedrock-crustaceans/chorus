use crate::block_definition::BlockDefinition;
use crate::block_id;
use crate::component::collision_box_component::CollisionBoxComponent;
use crate::component::light_emission_component::LightEmissionComponent;
use crate::component::map_color_component::MapColorComponent;
use crate::component::mineable_component::MineableComponent;
use crate::component::moveable_component::{MoveableComponent, Movement};
use crate::component::solid_component::SolidComponent;
use crate::const_block;

pub static END_GATEWAY: BlockDefinition = const_block! {
    identifier: block_id::END_GATEWAY,
    states: [],
    components: [
        MapColorComponent { r: 25, g: 25, b: 25, a: 255 },
        LightEmissionComponent { emission: 15 },
        MineableComponent::hardness(-1.0),
        MoveableComponent { movement: Movement::None, sticky: false },
        CollisionBoxComponent::enabled(false),
        SolidComponent { solid: false },
    ],
    permutations: [],
};
