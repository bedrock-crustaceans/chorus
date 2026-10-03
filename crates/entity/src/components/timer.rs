use bevy_ecs::prelude::Component;

#[derive(Component)]
pub struct Timer {
    // NBT fields
    time_stamp: i64,
    has_executed: bool,
    count_time: i32,
}
