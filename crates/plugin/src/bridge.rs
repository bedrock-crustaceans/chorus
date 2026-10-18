//! What the host side has to provide for plugin imports to do anything.
//!
//! This crate owns the generated import traits but not the game, so the traits forward to a
//! `HostBridge`. The implementation lives in `chorus::plugin`, which is where the erased world
//! pointer becomes a `&mut World` again. Tests get [`NoopBridge`].

use crate::chorus::plugin::command::Definition;
use crate::chorus::plugin::common::LogLevel;
use crate::chorus::plugin::player::Player;

/// Every method takes the erased world pointer the dispatch set, so the bridge is stateless and
/// can be shared between every loaded plugin.
pub trait HostBridge: Send + Sync {
    fn log(&self, world: *mut (), plugin: &str, level: LogLevel, message: &str);

    fn player_by_name(&self, world: *mut (), name: &str) -> Option<Player>;

    fn all_players(&self, world: *mut ()) -> Vec<Player>;

    fn send_message(&self, world: *mut (), player: &str, message: &str) -> Result<(), String>;

    fn broadcast(&self, world: *mut (), message: &str);

    fn register_command(&self, world: *mut (), plugin: &str, definition: Definition) -> Result<u32, String>;
}

/// A bridge that does nothing, for the metadata probe instance and for tests that only care
/// whether a component loads.
pub struct NoopBridge;

impl HostBridge for NoopBridge {
    fn log(&self, _world: *mut (), plugin: &str, level: LogLevel, message: &str) {
        let _ = (plugin, level, message);
    }

    fn player_by_name(&self, _world: *mut (), _name: &str) -> Option<Player> {
        None
    }

    fn all_players(&self, _world: *mut ()) -> Vec<Player> {
        Vec::new()
    }

    fn send_message(&self, _world: *mut (), _player: &str, _message: &str) -> Result<(), String> {
        Err(String::from("this server has no plugin bridge installed"))
    }

    fn broadcast(&self, _world: *mut (), _message: &str) {}

    fn register_command(&self, _world: *mut (), _plugin: &str, _definition: Definition) -> Result<u32, String> {
        Err(String::from("this server has no plugin bridge installed"))
    }
}
