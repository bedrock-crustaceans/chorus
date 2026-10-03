use bevy_ecs::prelude::Component;

#[derive(Component)]
pub struct EntityVillager {
    // NBT fields
    willing: bool,
}
