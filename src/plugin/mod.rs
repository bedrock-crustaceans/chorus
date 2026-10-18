//! WASM plugins: the bridge between `chorus_plugin`'s ABI and this server.
//!
//! `chorus_plugin` owns the ABI and the wasmtime engine; this module owns everything that needs
//! the `World`. Plugins are loaded once, in `PostStartup`, so every registry and the level already
//! exist by the time a plugin's `on_load` runs.

pub mod command;
pub mod events;
pub mod host_impl;

use std::sync::{Arc, Mutex, MutexGuard};

use bevy_app::{App, PostStartup};
use bevy_ecs::prelude::{Mut, Resource, World};
use bevy_ecs::schedule::IntoScheduleConfigs;
use chorus_core::config::Config;
use chorus_plugin::host::PluginHost;
use tracing::{error, info};

use crate::plugin::host_impl::ChorusBridge;
use crate::server::Server;

/// The plugin host, as a Bevy resource.
///
/// The `Mutex` is not about contention: `WasiCtx` is `Send` but not `Sync`, and a Bevy resource has
/// to be both. Every plugin dispatch runs in an exclusive system on the main thread, so the lock is
/// uncontended and `lock` never actually blocks.
#[derive(Resource)]
pub struct PluginHostResource(Mutex<PluginHost>);

impl PluginHostResource {
    pub fn host(&self) -> MutexGuard<'_, PluginHost> {
        self.0.lock().expect("the plugin host mutex is never held across a panic")
    }
}

/// Installs the plugin host, unless `[plugins] enabled = false`.
pub struct PluginSystem;

impl bevy_app::Plugin for PluginSystem {
    fn build(&self, app: &mut App) {
        let config = app.world().resource::<Config>().plugins.clone();
        if !config.enabled {
            info!("plugins are disabled");
            return;
        }

        match PluginHost::new(&config, Arc::new(ChorusBridge)) {
            Ok(host) => {
                app.insert_resource(PluginHostResource(Mutex::new(host)));
                app.init_resource::<command::PluginCommands>();
                app.init_resource::<events::PendingPluginEvents>();
                app.add_systems(PostStartup, load_plugins.before(Server::start));
                app.add_systems(crate::Tick, events::collect_observed_events.in_set(crate::schedule::GameSet::Actors));
                app.add_systems(crate::Tick, events::dispatch_observed_events.in_set(crate::schedule::GameSet::Broadcast));
            }
            Err(err) => error!("could not start the plugin host, continuing without plugins: {err}"),
        }
    }
}

/// Loads every component in the plugin directory and calls `on_load` on each.
///
/// Exclusive because a plugin's `on_load` can call back into the world, and `resource_scope` is
/// what lets the host and the world be borrowed at the same time.
fn load_plugins(world: &mut World) {
    let loaded = world.resource_scope(|world, host: Mut<PluginHostResource>| {
        let mut host = host.host();
        let count = host.load_directory();
        if count > 0 {
            host.load_all(world as *mut World as *mut ());
        }
        (count, host.running_names())
    });

    let (count, running) = loaded;
    if count == 0 {
        return;
    }
    info!("loaded {count} plugins");

    // only plugins that survived `on_load` get their staged commands
    command::commit_plugin_commands(world, &running);
}

/// Gives every plugin its `on_unload` call. Run on a clean shutdown, before the level is saved.
pub fn unload_plugins(world: &mut World) {
    if !world.contains_resource::<PluginHostResource>() {
        return;
    }
    world.resource_scope(|world, host: Mut<PluginHostResource>| {
        host.host().unload_all(world as *mut World as *mut ());
    });
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU32, Ordering};

    use bevy_app::App;
    use chorus_core::config::Config;

    use super::*;

    fn fixture() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples/plugins/hello/target/wasm32-wasip2/release/hello.wasm")
    }

    static NEXT_DIR: AtomicU32 = AtomicU32::new(0);

    fn scratch_dir() -> PathBuf {
        let unique = NEXT_DIR.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("chorus-plugin-app-test-{}-{unique}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("scratch dir");
        dir
    }

    fn app_with(config: Config) -> App {
        let mut app = App::new();
        app.insert_resource(config);
        let mut registry = crate::registry::command_registry::CommandRegistry::new();
        registry.register_all(crate::command::r#impl::DEFINITIONS.iter().copied());
        app.insert_resource(registry);
        app.add_plugins(PluginSystem);
        app
    }

    #[test]
    fn a_disabled_config_installs_no_host() {
        let mut config = Config::default();
        config.plugins.enabled = false;

        let app = app_with(config);

        assert!(app.world().get_resource::<PluginHostResource>().is_none());
    }

    #[test]
    #[ignore]
    fn loads_a_plugin_at_startup() {
        let dir = scratch_dir();
        std::fs::copy(fixture(), dir.join("hello.wasm")).expect("fixture built; see examples/plugins/README.md");
        let mut config = Config::default();
        config.plugins.directory = dir;

        let mut app = app_with(config);
        app.update();

        let host = app.world().resource::<PluginHostResource>();
        let loaded: Vec<(String, bool)> = host.host().plugins().map(|plugin| (plugin.metadata.name.clone(), plugin.is_running())).collect();

        assert_eq!(loaded, vec![(String::from("hello"), true)]);
    }

    /// An app with the example plugin loaded and its `on_load` already run.
    fn loaded_app() -> App {
        let dir = scratch_dir();
        std::fs::copy(fixture(), dir.join("hello.wasm")).expect("fixture built; see examples/plugins/README.md");
        let mut config = Config::default();
        config.plugins.directory = dir;

        let mut app = app_with(config);
        app.update();
        assert!(
            app.world().resource::<PluginHostResource>().host().plugins().all(|plugin| plugin.is_running()),
            "the fixture should load cleanly"
        );
        app
    }

    fn chat(entity: bevy_ecs::prelude::Entity, message: &str) -> crate::chat::PlayerChatMessage {
        crate::chat::PlayerChatMessage {
            entity,
            sender_name: String::from("Steve"),
            sender_xuid: String::from("1"),
            message: message.to_owned(),
        }
    }

    /// The example plugin cancels anything containing `badword`.
    #[test]
    #[ignore]
    fn a_plugin_can_cancel_chat() {
        let mut app = loaded_app();
        let entity = app.world_mut().spawn_empty().id();

        let outcome = events::moderate(app.world_mut(), &chat(entity, "say badword out loud"));

        assert_eq!(outcome, Some((String::from("say badword out loud"), true)));
    }

    /// And rewrites `hug` so the change has to survive the round trip back out of wasm.
    #[test]
    #[ignore]
    fn a_plugin_can_rewrite_chat() {
        let mut app = loaded_app();
        let entity = app.world_mut().spawn_empty().id();

        let outcome = events::moderate(app.world_mut(), &chat(entity, "need a hug"));

        assert_eq!(outcome, Some((String::from("need a §dhug§r"), false)));
    }

    #[test]
    #[ignore]
    fn chat_a_plugin_leaves_alone_comes_back_unchanged() {
        let mut app = loaded_app();
        let entity = app.world_mut().spawn_empty().id();

        let outcome = events::moderate(app.world_mut(), &chat(entity, "hello everyone"));

        assert_eq!(outcome, Some((String::from("hello everyone"), false)));
    }

    /// A plugin that is out of the rotation gets no say: the message goes out as it was typed.
    #[test]
    #[ignore]
    fn a_quarantined_plugin_leaves_chat_alone() {
        let mut app = loaded_app();
        app.world_mut().resource_mut::<PluginHostResource>().host().quarantine("hello", "test");
        let entity = app.world_mut().spawn_empty().id();

        let outcome = events::moderate(app.world_mut(), &chat(entity, "say badword out loud"));

        assert_eq!(outcome, None);
    }

    #[test]
    #[ignore]
    fn an_observed_event_does_not_quarantine_the_plugin() {
        let mut app = loaded_app();
        let player = chorus_plugin::chorus::plugin::player::Player {
            xuid: String::from("1"),
            name: String::from("Steve"),
            position: chorus_plugin::chorus::plugin::common::Position { x: 0.0, y: 0.0, z: 0.0 },
            game_mode: chorus_plugin::chorus::plugin::common::GameMode::Survival,
        };
        app.world_mut()
            .resource_mut::<events::PendingPluginEvents>()
            .queue(chorus_plugin::chorus::plugin::event::Event::PlayerJoin(chorus_plugin::chorus::plugin::event::PlayerJoinData { player }));

        events::dispatch_observed_events(app.world_mut());

        assert!(app.world().resource::<PluginHostResource>().host().plugins().all(|plugin| plugin.is_running()));
    }

    /// The whole chain: the plugin registered `/hello`, so dispatching it has to run wasm and come
    /// back without the plugin being quarantined.
    #[test]
    #[ignore]
    fn a_plugin_command_runs_from_the_console() {
        let mut app = loaded_app();
        let console = app.world_mut().spawn(crate::console::Console).id();

        crate::registry::command_registry::CommandRegistry::dispatch(&mut crate::command::context::CommandContext::new(app.world_mut(), console), "hello");

        assert!(app.world().resource::<PluginHostResource>().host().plugins().all(|plugin| plugin.is_running()));
    }

    /// The example plugin registers `/hello` from `on_load`, so a clean load has to leave it in the
    /// real command registry.
    #[test]
    #[ignore]
    fn a_plugins_command_reaches_the_registry() {
        let dir = scratch_dir();
        std::fs::copy(fixture(), dir.join("hello.wasm")).expect("fixture built; see examples/plugins/README.md");
        let mut config = Config::default();
        config.plugins.directory = dir;

        let mut app = app_with(config);
        app.update();

        assert!(app.world().resource::<crate::registry::command_registry::CommandRegistry>().get("hello").is_some());
    }
}
