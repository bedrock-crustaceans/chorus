use crate::config::Config;
use crate::logger::setup_logger;
use crate::server::Server;
use bevy_app::{App, AppExit, Plugin, PluginsState, PreStartup, RunFixedMainLoop, TaskPoolOptions, TaskPoolPlugin, TaskPoolThreadAssignmentPolicy};
use bevy_ecs::schedule::{IntoScheduleConfigs, Schedule};
use chorus_core::schedule::run_fixed_tick;
use std::time::Instant;
use tracing::{info, warn};

pub mod command;
pub mod console;
pub mod form;
pub mod logger;
pub mod network;
pub mod player;
pub mod registry;
pub mod resource;
pub mod server;

pub use chorus_block as block;
pub use chorus_core::schedule::{JobQueue, Tick, TickClock, TickSet};
pub use chorus_core::{config, info, math};
pub use chorus_entity as entity;
pub use chorus_item as item;

pub mod level {
    pub use chorus_level::*;

    pub mod generator {
        pub use chorus_level::generator::*;
        pub use chorus_worldgen as r#impl;
    }
}

pub mod utils {
    pub use chorus_block::hash_utils;
    pub use chorus_core::utils::*;
}

pub struct LoopRunner;

impl LoopRunner {
    pub fn run(mut app: App) -> AppExit {
        let plugins_state = app.plugins_state();
        if plugins_state != PluginsState::Cleaned {
            while app.plugins_state() == PluginsState::Adding {
                bevy_tasks::tick_global_task_pools_on_main_thread();
            }
            app.finish();
            app.cleanup();
        }

        loop {
            app.update();

            if let Some(exit) = app.should_exit() {
                if let Some(mut level) = app.world_mut().get_resource_mut::<level::Level>() {
                    let saved = level.save();
                    info!("saved {saved} chunks before exiting");
                }
                return exit;
            }

            let deadline = Instant::now() + app.world().resource::<TickClock>().remaining();

            while Instant::now() < deadline {
                let Some(job) = app.world_mut().resource_mut::<JobQueue>().pop() else {
                    break;
                };
                if let Err(err) = app.world_mut().run_system(job) {
                    warn!("job failed to run: {err}");
                }
            }

            if let Some(remaining) = deadline.checked_duration_since(Instant::now()) {
                spin_sleep::sleep(remaining);
            }
        }
    }
}

fn thread_policy(max_threads: usize, percent: f32) -> TaskPoolThreadAssignmentPolicy {
    TaskPoolThreadAssignmentPolicy {
        min_threads: 0,
        max_threads,
        percent,
        on_thread_spawn: None,
        on_thread_destroy: None,
    }
}

pub struct ChorusPlugin;

impl Plugin for ChorusPlugin {
    fn build(&self, app: &mut App) {
        let config = Config::setup();

        app.add_schedule(Schedule::new(Tick));
        app.configure_sets(Tick, (TickSet::First, TickSet::Update, TickSet::Last).chain());

        app.init_resource::<TickClock>().init_resource::<JobQueue>().add_systems(RunFixedMainLoop, run_fixed_tick);

        if !app.is_plugin_added::<TaskPoolPlugin>() {
            app.add_plugins(TaskPoolPlugin {
                task_pool_options: TaskPoolOptions {
                    min_total_threads: config.threads,
                    max_total_threads: config.threads,
                    io: thread_policy(2, 0.1),
                    async_compute: thread_policy(usize::MAX, 0.75),
                    compute: thread_policy(usize::MAX, 1.0),
                },
            });
        }

        app.insert_resource(config).add_systems(PreStartup, setup_logger).add_plugins(Server);
    }
}

pub struct Chorus;

impl Chorus {
    pub fn init() -> App {
        let mut app = App::new();
        app.add_plugins(ChorusPlugin).set_runner(LoopRunner::run);
        app
    }
}
