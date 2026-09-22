use bevy_ecs::prelude::Component;

#[derive(Component)]
pub struct EntityAbstractArrow {
    // NBT fields
    is_creative: bool,
    owner_id: i64,
    player: bool,
}
