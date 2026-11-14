use bevy_ecs::prelude::Component;

#[derive(Component, Default)]
pub struct Ageable {
    age: i32,
}
