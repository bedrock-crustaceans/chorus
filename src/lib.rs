use crate::config::Config;
use crate::logger::setup_logger;
use crate::server::{Server, TICK_RATE};
use bevy_app::{App, AppExit, Plugin, PluginsState, PreStartup, RunFixedMainLoop, TaskPoolOptions, TaskPoolPlugin, TaskPoolThreadAssignmentPolicy};
use bevy_ecs::resource::Resource;
use bevy_ecs::schedule::{IntoScheduleConfigs, Schedule, ScheduleLabel, SystemSet};
use bevy_ecs::system::SystemId;
use bevy_ecs::world::World;
use std::collections::VecDeque;
use std::time::{Duration, Instant};
use tracing::warn;

pub mod block;
pub mod command;
pub mod config;
pub mod entity;
pub mod error;
pub mod form;
pub mod info;
pub mod item;
pub mod level;
pub mod logger;
pub mod math;
pub mod network;
pub mod player;
pub mod registry;
pub mod resource;
pub mod server;
pub mod utils;

#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Tick;

#[derive(SystemSet, Clone, Debug, PartialEq, Eq, Hash)]
pub enum TickSet {
    First,
    Update,
    Last,
}

#[derive(Resource, Debug)]
pub struct TickClock {
    pub interval: Duration,
    accumulated: Duration,
    last_instant: Instant,
}

impl Default for TickClock {
    fn default() -> Self {
        Self {
            interval: Duration::from_secs_f64(1.0 / TICK_RATE),
            accumulated: Duration::ZERO,
            last_instant: Instant::now(),
        }
    }
}

impl TickClock {
    pub fn remaining(&self) -> Duration {
        self.interval.saturating_sub(self.accumulated)
    }
}

#[derive(Resource, Default)]
pub struct JobQueue(VecDeque<SystemId>);

impl JobQueue {
    pub fn push(&mut self, job: SystemId) {
        self.0.push_back(job);
    }
}

const MAX_TICKS_PER_UPDATE: u32 = 5;

fn run_fixed_tick(world: &mut World) {
    let now = Instant::now();

    let (tick_duration, mut accumulated) = {
        let mut clock = world.resource_mut::<TickClock>();
        let elapsed = now - clock.last_instant;
        clock.accumulated += elapsed;
        clock.last_instant = now;
        (clock.interval, clock.accumulated)
    };

    let mut ticks_run = 0u32;
    while accumulated >= tick_duration && ticks_run < MAX_TICKS_PER_UPDATE {
        accumulated -= tick_duration;
        world.run_schedule(Tick);
        ticks_run += 1;
    }

    world.resource_mut::<TickClock>().accumulated = if ticks_run == MAX_TICKS_PER_UPDATE { Duration::ZERO } else { accumulated };
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
                return exit;
            }

            let deadline = Instant::now() + app.world().resource::<TickClock>().remaining();

            while Instant::now() < deadline {
                let Some(job) = app.world_mut().resource_mut::<JobQueue>().0.pop_front() else {
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
