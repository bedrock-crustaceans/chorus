pub use chorus_core::schedule::*;

use bevy_ecs::schedule::SystemSet;

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GameSet {
    Connection,
    Input,
    World,
    Fluids,
    Actors,
    Chat,
    Chunks,
    Broadcast,
}
