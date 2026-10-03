use bevy_ecs::prelude::Component;

#[derive(Component)]
pub struct EntityThrowable {
    // NBT fields
    in_ground: bool,
    owner_id: i64,
    shake: bool,
}
