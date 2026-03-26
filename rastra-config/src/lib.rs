pub mod config;

use log::debug;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::process::exit;
use rastra_api::SERVER_NAME;

const CONFIG_PATH: &str = "rastra.toml";

#[derive(Serialize, Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct RAstraConfig {
    pub ip: String,
    pub port: u32,
    pub motd: String,
    pub sub_motd: String,
    pub max_players: u32,
    pub threads: usize,
    pub log_to_file: bool,
    pub logs_directory: PathBuf,
    pub resource_packs_directory: PathBuf,
    pub behavior_packs_directory: PathBuf,
    pub level_name: String,
    pub level_seed: u64,
}

impl Default for RAstraConfig {
    fn default() -> Self {
        Self {
            ip: String::from("127.0.0.1"),
            port: 19132,
            motd: String::from(SERVER_NAME.to_owned() + " Server"),
            sub_motd: String::from("Powered by bedrock-rs"),
            max_players: 50,
            threads: 4,
            log_to_file: true,
            logs_directory: PathBuf::from("logs"),
            resource_packs_directory: PathBuf::from("resource_packs"),
            behavior_packs_directory: PathBuf::from("behavior_packs"),
            level_name: String::from("Bedrock Level"),
            level_seed: 0,
        }
    }
}

pub fn setup_config() -> RAstraConfig {
    let config = if PathBuf::from(CONFIG_PATH).exists() {
        let text = fs::read_to_string(CONFIG_PATH).unwrap_or_else(|err| {
            eprintln!(
                "An unexpected Error occurred while trying to read {CONFIG_PATH:?}, Err: {err}"
            );
            exit(1);
        });

        toml::from_str(&text).unwrap_or_else(|err| {
            eprintln!("An unexpected Error occurred while trying to deserialize {CONFIG_PATH:?}, Err: {err}");
            exit(1);
        })
    } else {
        let config = RAstraConfig::default();

        let text = toml::to_string(&config).unwrap_or_else(|err| {
            eprintln!(
                "An unexpected Error occurred while trying to serialize {config:?}, Err: {err}"
            );
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
            eprintln!("An unexpected Error occurred while trying to create the resource packs directory at {:?}, Err: {err}", config.resource_packs_directory);
            exit(1)
        });
    };

    if !&config.behavior_packs_directory.exists() {
        fs::create_dir(&config.behavior_packs_directory).unwrap_or_else(|err| {
            eprintln!("An unexpected Error occurred while trying to create the behavior packs directory at {:?}, Err: {err:?}", config.behavior_packs_directory);
            exit(1)
        });
    };

    debug!("Config read!");

    config
}
