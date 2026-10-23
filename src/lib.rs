use crate::config::Config;
use crate::logger::setup_logger;
use crate::server::Server;
use bevy_app::{App, AppExit, Plugin, PluginsState, PreStartup, RunFixedMainLoop, TaskPoolOptions, TaskPoolPlugin, TaskPoolThreadAssignmentPolicy};
use bevy_ecs::schedule::{IntoScheduleConfigs, Schedule};
use chorus_core::schedule::{run_fixed_tick, run_tasks};
use std::time::{Duration, Instant};
use tracing::{info, warn};

pub mod actor;
pub mod chat;
pub mod command;
pub mod console;
pub mod debug;
pub mod form;
pub mod format;
pub mod logger;
pub mod network;
pub mod player;
pub mod registry;
pub mod resource;
pub mod schedule;
pub mod server;
pub mod world;

pub use chorus_block as block;
pub use chorus_core::schedule::{Budget, Priority, Scheduler, Tick, TickClock, TickSet, Wake};
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

#[derive(bevy_ecs::prelude::Resource, Default)]
pub struct ShutdownState {
    pub done: bool,
}

pub fn shutdown(world: &mut bevy_ecs::prelude::World) {
    let already = world
        .get_resource::<ShutdownState>()
        .map(|state| state.done)
        .unwrap_or(false);
    if already {
        return;
    }
    if let Some(mut state) = world.get_resource_mut::<ShutdownState>() {
        state.done = true;
    } else {
        world.insert_resource(ShutdownState { done: true });
    }

    let _tag = tracing::info_span!("shutdown").entered();
    let compact = world.get_resource::<Config>().is_some_and(|config| config.level.database.shutdown_compaction);
    actor::storage::save_all_actors(world);
    if let Some(mut level) = world.get_resource_mut::<level::Level>() {
        let saved = level.save_blocking();
        info!("saved {saved} chunks before exiting");
        if compact && let Some(storage) = level.storage() {
            info!("compacting the level database before exiting");
            if let Err(err) = storage.compact_blocking() {
                warn!("failed to compact the level database: {err}");
            }
        }
    }
    console::shutdown();
}

pub struct ShutdownPlugin;

impl Plugin for ShutdownPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(bevy_app::Last, save_on_exit);
    }
}

fn save_on_exit(mut exits: bevy_ecs::prelude::MessageReader<AppExit>, mut commands: bevy_ecs::prelude::Commands) {
    if exits.read().next().is_some() {
        commands.queue(|world: &mut bevy_ecs::prelude::World| shutdown(world));
    }
}

#[derive(bevy_ecs::prelude::Resource, Clone, Copy, Debug, Default)]
pub struct Embedded;

#[derive(bevy_ecs::prelude::Resource, Clone, Copy, Debug, PartialEq, Eq)]
pub enum SchedulerAuthority {
    LoopRunner,
    Driver,
}

#[derive(bevy_ecs::prelude::Resource, Clone, Copy, Debug)]
pub struct SchedulerControl {
    pub authority: SchedulerAuthority,
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

        let external_scheduler = app
            .world()
            .get_resource::<SchedulerControl>()
            .map(|control| control.authority == SchedulerAuthority::Driver)
            .unwrap_or(false);

        loop {
            app.update();

            if let Some(exit) = app.should_exit() {
                shutdown(app.world_mut());
                return exit;
            }

            let gap_started = Instant::now();
            let deadline = gap_started + app.world().resource::<TickClock>().remaining();
            let mut slept = Duration::ZERO;

            if external_scheduler {
                // Task driving is owned by the embedded scheduler driver.
                // Only pace the loop to the tick deadline to avoid busy-waiting.
                let now = Instant::now();
                if now < deadline {
                    spin_sleep::sleep(deadline.saturating_duration_since(now));
                    slept = deadline.saturating_duration_since(gap_started);
                }
            } else {
                loop {
                    run_tasks(app.world_mut(), deadline);
                    let now = Instant::now();
                    if now >= deadline {
                        break;
                    }
                    match app.world_mut().resource_mut::<Scheduler>().next_wake() {
                        Some(wake) if wake < deadline => std::thread::sleep(wake.saturating_duration_since(now)),
                        _ => spin_sleep::sleep(deadline.saturating_duration_since(now)),
                    }
                    slept += now.elapsed();
                }
            }
            app.world_mut().resource_mut::<Scheduler>().record_gap(gap_started.elapsed(), slept);
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

pub struct ChorusPlugin {
    pub config: Option<Config>,
    pub schedule_driver: bool,
    pub console: bool,
}

impl Default for ChorusPlugin {
    fn default() -> Self {
        Self {
            config: None,
            schedule_driver: true,
            console: true,
        }
    }
}

impl ChorusPlugin {
    pub fn embedded() -> Self {
        Self {
            console: false,
            ..Default::default()
        }
    }

    pub fn headless() -> Self {
        Self {
            schedule_driver: false,
            ..Default::default()
        }
    }

    pub fn with_config(mut self, config: Config) -> Self {
        self.config = Some(config);
        self
    }
}

impl Plugin for ChorusPlugin {
    fn build(&self, app: &mut App) {
        let config = self.config.clone().unwrap_or_else(Config::setup);

        app.init_resource::<ShutdownState>();

        let authority = if self.schedule_driver {
            SchedulerAuthority::Driver
        } else {
            SchedulerAuthority::LoopRunner
        };
        app.insert_resource(SchedulerControl { authority });

        if self.schedule_driver {
            app.insert_resource(Embedded);
        }

        app.add_schedule(Schedule::new(Tick));
        app.configure_sets(Tick, (TickSet::First, TickSet::Update, TickSet::Last).chain());
        app.configure_sets(
            Tick,
            (
                schedule::GameSet::Connection,
                schedule::GameSet::Input,
                schedule::GameSet::World,
                schedule::GameSet::Actors,
                schedule::GameSet::Chat,
                schedule::GameSet::Chunks,
                schedule::GameSet::Broadcast,
            )
                .chain()
                .in_set(TickSet::Update),
        );

        app.init_resource::<TickClock>().init_resource::<Scheduler>().add_systems(RunFixedMainLoop, run_fixed_tick);

        if !app.is_plugin_added::<TaskPoolPlugin>() {
            app.add_plugins(TaskPoolPlugin {
                task_pool_options: TaskPoolOptions {
                    min_total_threads: config.server.threads.count(),
                    max_total_threads: config.server.threads.count(),
                    io: thread_policy(2, 0.1),
                    async_compute: thread_policy(usize::MAX, 0.75),
                    compute: thread_policy(usize::MAX, 1.0),
                },
            });
        }

        app.insert_resource(config).add_systems(PreStartup, setup_logger).add_plugins(Server);

        if self.schedule_driver {
            app.add_systems(bevy_app::Last, drive_scheduler);
        }
        // Shutdown handling is shared between headless and embedded runners.
        app.add_plugins(ShutdownPlugin);
        if self.console {
            app.add_plugins(console::ConsolePlugin::default());
        } else {
            app.add_plugins(console::ConsolePlugin::embedded());
        }
    }
}

fn drive_scheduler(world: &mut bevy_ecs::prelude::World) {
    // Only the configured owner drives the scheduler; prevents double run_tasks.
    let owner_is_driver = world
        .get_resource::<SchedulerControl>()
        .map(|control| control.authority == SchedulerAuthority::Driver)
        .unwrap_or(true);
    if !owner_is_driver {
        return;
    }
    let deadline = Instant::now() + Duration::from_millis(10);
    chorus_core::schedule::run_tasks(world, deadline);
}

pub struct Chorus;

impl Chorus {
    pub fn init() -> App {
        let mut app = App::new();
        app.add_plugins(ChorusPlugin::headless()).set_runner(LoopRunner::run);
        app
    }
}
