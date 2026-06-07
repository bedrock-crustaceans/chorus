use bevy_ecs::prelude::Resource;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::process::exit;
use tracing::debug;

const CONFIG_PATH: &str = "chorus.toml";

#[derive(Resource, Serialize, Deserialize, Clone, Debug)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    pub ip: String,
    pub port: u16,
    pub name: String,
    pub sub_name: String,
    pub max_players: i32,
    pub threads: usize,
    pub log_to_file: bool,
    pub logs_directory: PathBuf,
    pub resource_packs_directory: PathBuf,
    pub behavior_packs_directory: PathBuf,
    pub level_name: String,
    pub level_seed: u64,
    pub online_mode: bool,
    pub encryption: bool,
    pub log_level: String,
    pub force_accept_resource_packs: bool,
    pub force_disable_vibrant_visuals: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            ip: String::from("0.0.0.0"),
            port: 19132,
            name: String::from("Chorus"),
            sub_name: String::from("bedrock-crustaceans.org"),
            max_players: 20,
            threads: 4,
            log_to_file: true,
            logs_directory: PathBuf::from("logs"),
            resource_packs_directory: PathBuf::from("resource_packs"),
            behavior_packs_directory: PathBuf::from("behavior_packs"),
            level_name: String::from("world"),
            level_seed: 0,
            online_mode: true,
            encryption: false,
            log_level: String::from("info"),
            force_accept_resource_packs: false,
            force_disable_vibrant_visuals: false,
        }
    }
}

impl Config {
    pub fn setup() -> Self {
        let config = if PathBuf::from(CONFIG_PATH).exists() {
            let text = fs::read_to_string(CONFIG_PATH).unwrap_or_else(|err| {
                eprintln!("An unexpected Error occurred while trying to read {CONFIG_PATH:?}, Err: {err}");
                exit(1);
            });

            toml::from_str(&text).unwrap_or_else(|err| {
                eprintln!("An unexpected Error occurred while trying to deserialize {CONFIG_PATH:?}, Err: {err}");
                exit(1);
            })
        } else {
            let config = Config::default();

            let text = toml::to_string(&config).unwrap_or_else(|err| {
                eprintln!("An unexpected Error occurred while trying to serialize {config:?}, Err: {err}");
                exit(1);
            });

            fs::write(CONFIG_PATH, text).unwrap_or_else(|err| {
                eprintln!("An unexpected Error occurred while trying to write the missing config to {CONFIG_PATH:?}, Err: {err}");
            });

            config
        };

        if !&config.logs_directory.exists() {
            fs::create_dir(&config.logs_directory).unwrap_or_else(|err| {
                eprintln!("An unexpected Error occurred while trying to create the logs directory at {:?}, Err: {err}", config.logs_directory);
                exit(1)
            });
        };

        if !&config.resource_packs_directory.exists() {
            fs::create_dir(&config.resource_packs_directory).unwrap_or_else(|err| {
                eprintln!(
                    "An unexpected Error occurred while trying to create the resource packs directory at {:?}, Err: {err}",
                    config.resource_packs_directory
                );
                exit(1)
            });
        };

        if !&config.behavior_packs_directory.exists() {
            fs::create_dir(&config.behavior_packs_directory).unwrap_or_else(|err| {
                eprintln!(
                    "An unexpected Error occurred while trying to create the behavior packs directory at {:?}, Err: {err:?}",
                    config.behavior_packs_directory
                );
                exit(1)
            });
        };

        debug!("Config read!");

        config
    }
}
