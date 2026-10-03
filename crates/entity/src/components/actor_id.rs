use bevy_ecs::prelude::Component;

#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct ActorId {
    pub unique_id: i64,
    pub runtime_id: u64,
}
