use bevy_ecs::schedule::SystemSet;

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GameSet {
    Connection,
    Input,
    World,
    Actors,
    Chat,
    Chunks,
    Broadcast,
}
