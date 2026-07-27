use crate::config::Config;
use crate::logger::setup_logger;
use crate::server::Server;
use bevy_app::{App, PreStartup, ScheduleRunnerPlugin, TaskPoolOptions, TaskPoolPlugin};
use bevy_time::TimePlugin;

pub mod block;
pub mod command;
pub mod config;
pub mod entity;
pub mod error;
pub mod form;
pub mod info;
pub mod inventory;
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
            .add_plugins(ScheduleRunnerPlugin::default())
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
