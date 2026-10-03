use bevy_ecs::prelude::Component;

#[derive(Component)]
pub struct Breedable {
    in_love: i32,
    love_cause: i64,
    breed_cooldown: i32,
}

impl Breedable {
    pub fn default() -> Self {
        Self {
            in_love: 0,
            love_cause: 0,
            breed_cooldown: 0,
        }
    }
}
