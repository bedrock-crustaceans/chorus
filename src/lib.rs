use crate::config::Config;
use crate::logger::setup_logger;
use crate::server::{Server, TICK_RATE};
use bevy_app::{App, PreStartup, ScheduleRunnerPlugin, TaskPoolOptions, TaskPoolPlugin};
use bevy_time::TimePlugin;
use std::time::Duration;

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

pub struct Chorus;

impl Chorus {
    pub fn init() -> App {
        let config = Config::setup();

        let mut app = App::new();
        app.add_plugins(TimePlugin)
            // the default runner never sleeps, which burns every core even with nobody connected.
            // run it a few times per tick so Time<Fixed> stays accurate without spinning
            .add_plugins(ScheduleRunnerPlugin::run_loop(Duration::from_secs_f64(1. / (TICK_RATE * 5.))))
            .add_plugins(TaskPoolPlugin {
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
