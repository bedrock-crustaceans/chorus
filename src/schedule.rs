use bevy_ecs::schedule::SystemSet;

/// Stages of `TickSet::Update`, run in this order: packets are decoded into messages first, game
/// systems react to them, and what changed is sent to clients last.
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GameSet {
    /// Session state, login and the per-state packet handlers.
    Connection,
    /// Player input decoded from packets, commands.
    Input,
    /// Blocks and the level.
    World,
    /// Entities other than players.
    Actors,
    Chat,
    /// Chunk streaming and unloading.
    Chunks,
    /// Level changes sent to clients.
    Broadcast,
}
