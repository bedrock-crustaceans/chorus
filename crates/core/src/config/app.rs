use super::{Config, ConfigSection};
use bevy_app::{App, Plugin, Startup};
use bevy_ecs::prelude::Res;
use std::process::exit;
use tracing::warn;

pub struct ConfigPlugin {
    pub config: Option<Config>,
}

impl Plugin for ConfigPlugin {
    fn build(&self, app: &mut App) {
        let config = self.config.clone().unwrap_or_else(Config::setup);

        app.insert_resource(config).add_systems(Startup, finish);
    }
}

pub trait AppConfigExt {
    fn init_config<T: ConfigSection>(&mut self) -> &mut Self;
}

impl AppConfigExt for App {
    fn init_config<T: ConfigSection>(&mut self) -> &mut Self {
        let Some(mut config) = self.world_mut().get_resource_mut::<Config>() else {
            panic!("config sections are registered after ConfigPlugin, add it (or ChorusPlugin) first");
        };
        let section = config.register::<T>().unwrap_or_else(|err| {
            eprintln!("Failed to load the config: {err}");
            exit(1);
        });

        self.insert_resource(section)
    }
}

fn finish(config: Res<Config>) {
    for name in config.unregistered() {
        warn!("config section [{name}] is not read by any plugin");
    }
    config.create_missing();
}
