//! The generated import traits, implemented by forwarding to the [`HostBridge`].
//!
//! [`HostBridge`]: crate::bridge::HostBridge

use tracing::warn;

use crate::chorus::plugin::command::{Definition, Host as CommandHost};
use crate::chorus::plugin::common::LogLevel;
use crate::chorus::plugin::logging::Host as LoggingHost;
use crate::chorus::plugin::player::{Host as PlayerHost, Player};
use crate::chorus::plugin::text::Host as TextHost;
use crate::state::HostState;

/// Pulls the world pointer out, or logs and bails with `$fallback`.
///
/// A plugin cannot actually reach an import outside a dispatch, so this is a guard rail rather
/// than a path anyone takes.
macro_rules! world {
    ($self:expr) => {
        match $self.world_ptr() {
            Ok(world) => world,
            Err(err) => {
                warn!(plugin = $self.name(), "{err}");
                return;
            }
        }
    };
    ($self:expr, $fallback:expr) => {
        match $self.world_ptr() {
            Ok(world) => world,
            Err(err) => {
                warn!(plugin = $self.name(), "{err}");
                return $fallback;
            }
        }
    };
}

impl LoggingHost for HostState {
    fn log(&mut self, level: LogLevel, message: String) {
        let world = world!(self);
        let bridge = self.bridge().clone();
        let name = self.name().to_owned();
        bridge.log(world, &name, level, &message);
    }
}

impl TextHost for HostState {
    fn send_message(&mut self, player_name: String, message: String) -> Result<(), String> {
        let world = world!(self, Err(String::from("no world")));
        let bridge = self.bridge().clone();
        bridge.send_message(world, &player_name, &message)
    }

    fn broadcast(&mut self, message: String) {
        let world = world!(self);
        let bridge = self.bridge().clone();
        bridge.broadcast(world, &message);
    }
}

impl PlayerHost for HostState {
    fn get_by_name(&mut self, name: String) -> Option<Player> {
        let world = world!(self, None);
        let bridge = self.bridge().clone();
        bridge.player_by_name(world, &name)
    }

    fn all(&mut self) -> Vec<Player> {
        let world = world!(self, Vec::new());
        let bridge = self.bridge().clone();
        bridge.all_players(world)
    }
}

impl CommandHost for HostState {
    fn register(&mut self, definition: Definition) -> Result<u32, String> {
        let world = world!(self, Err(String::from("no world")));
        let bridge = self.bridge().clone();
        let name = self.name().to_owned();
        bridge.register_command(world, &name, definition)
    }
}

/// Marker telling the generated `add_to_linker` functions what the host state looks like.
struct HostData;

impl wasmtime::component::HasData for HostData {
    type Data<'a> = &'a mut HostState;
}

/// Wires every `chorus:plugin` import onto a linker.
///
/// Behaviour comes from the [`HostBridge`] in the store, so this is the same call whether the
/// server is running for real or a test installed a stub bridge.
///
/// [`HostBridge`]: crate::bridge::HostBridge
pub fn add_to_linker(linker: &mut wasmtime::component::Linker<HostState>) -> wasmtime::Result<()> {
    use crate::chorus::plugin::{command, logging, player, text};

    logging::add_to_linker::<HostState, HostData>(linker, |state| state)?;
    text::add_to_linker::<HostState, HostData>(linker, |state| state)?;
    player::add_to_linker::<HostState, HostData>(linker, |state| state)?;
    command::add_to_linker::<HostState, HostData>(linker, |state| state)?;
    Ok(())
}
