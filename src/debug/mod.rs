pub mod command;
pub mod hud;

use crate::registry::command_registry::CommandRegistry;
use bevy_app::{App, Plugin, Startup};
use bevy_ecs::prelude::{IntoScheduleConfigs, ResMut};

pub use command::DEBUG_COMMAND;

pub struct DebugPlugin;

impl Plugin for DebugPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(hud::DebugHudPlugin).add_systems(Startup, DebugPlugin::register_command.after(CommandRegistry::init));
    }
}

impl DebugPlugin {
    fn register_command(mut registry: ResMut<CommandRegistry>) {
        registry.register(&DEBUG_COMMAND);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_plugin_registers_the_debug_command_once_the_registry_exists() {
        let mut app = App::new();
        app.add_systems(Startup, CommandRegistry::init).add_plugins(DebugPlugin);
        app.update();
        let registry = app.world().resource::<CommandRegistry>();
        assert!(registry.get("debug").is_some());
        assert!(registry.get("help").is_some());
    }
}
