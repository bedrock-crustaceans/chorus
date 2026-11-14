use bevy_ecs::prelude::Component;

#[derive(Component, Default)]
pub struct Breedable {
    in_love: i32,
    love_cause: i64,
    breed_cooldown: i32,
}
