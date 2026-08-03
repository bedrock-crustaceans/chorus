use crate::config::Config;
use crate::logger::setup_logger;
use crate::server::Server;
use bevy_app::{App, AppExit, PluginsState, PreStartup, TaskPoolOptions, TaskPoolPlugin};
use bevy_ecs::prelude::Resource;
use std::time::{Duration, Instant};

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

#[derive(Resource, Debug)]
pub struct SleepInterval(pub Duration);

impl Default for SleepInterval {
    fn default() -> Self {
        Self(Duration::from_millis(50))
    }
}

pub struct SleepRunner;

impl SleepRunner {
    pub fn run(mut app: App) -> AppExit {
        let plugins_state = app.plugins_state();
        if plugins_state != PluginsState::Cleaned {
            while app.plugins_state() == PluginsState::Adding {
                bevy_tasks::tick_global_task_pools_on_main_thread();
            }
            app.finish();
            app.cleanup();
        }

        let tick = move |app: &mut App| -> Result<Option<Duration>, AppExit> {
            let start_time = Instant::now();

            app.update();

            if let Some(exit) = app.should_exit() {
                return Err(exit);
            };

            let end_time = Instant::now();

            if let Some(&SleepInterval(tick_interval)) = app.world().get_resource::<SleepInterval>() {
                let exe_time = end_time - start_time;
                if exe_time < tick_interval {
                    return Ok(Some(tick_interval - exe_time));
                }
            }

            Ok(None)
        };

        loop {
            match tick(&mut app) {
                Ok(Some(delay)) => {
                    spin_sleep::sleep(delay);
                }
                Ok(None) => continue,
                Err(exit) => return exit,
            }
        }
    }
}

pub struct Chorus;

impl Chorus {
    pub fn init() -> App {
        let config = Config::setup();

        let mut app = App::new();

        app.init_resource::<SleepInterval>().set_runner(SleepRunner::run);

        app.add_plugins(TaskPoolPlugin {
            task_pool_options: TaskPoolOptions {
                max_total_threads: config.threads,
                min_total_threads: config.threads,
                ..Default::default()
            },
        })
        .insert_resource(config)
        .add_systems(PreStartup, setup_logger)
        .add_plugins(Server);
        app
    }
}
