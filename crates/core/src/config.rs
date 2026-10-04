use crate::permission::PermissionLevel;
use bevy_ecs::prelude::Resource;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::process::exit;
use std::time::Duration;
use tracing::debug;

const CONFIG_PATH: &str = "chorus.toml";

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum NetworkTransport {
    #[default]
    RakNet,
    NetherNet,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
#[serde(untagged)]
pub enum LevelSeed {
    Number(i64),
    Text(String),
}

impl Default for LevelSeed {
    fn default() -> Self {
        Self::Number(0)
    }
}

impl LevelSeed {
    pub fn value(&self) -> i64 {
        self.parse_with(Self::parse_text)
    }

    pub fn parse_with(&self, parse: impl FnOnce(&str) -> i64) -> i64 {
        match self {
            Self::Number(seed) => *seed,
            Self::Text(text) => parse(text),
        }
    }

    fn parse_text(text: &str) -> i64 {
        let text = text.trim();
        if text.is_empty() {
            return rand::random();
        }
        text.parse()
            .unwrap_or_else(|_| text.encode_utf16().fold(0i32, |hash, unit| hash.wrapping_mul(31).wrapping_add(unit as i32)) as i64)
    }
}

#[derive(Resource, Serialize, Deserialize, Clone, Debug, Default)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    pub server: ServerConfig,
    pub network: NetworkConfig,
    pub level: LevelConfig,
    pub packs: PackConfig,
    pub log: LogConfig,
    pub permissions: PermissionConfig,
}

/// Command permission levels: `default` for everyone, and `players` for specific players by name
/// or XUID, for example `players = { Steve = "admin" }`. The console is always `owner`.
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default, deny_unknown_fields)]
pub struct PermissionConfig {
    pub default: PermissionLevel,
    pub players: std::collections::BTreeMap<String, PermissionLevel>,
}

impl Default for PermissionConfig {
    fn default() -> Self {
        Self {
            default: PermissionLevel::Operator,
            players: Default::default(),
        }
    }
}

impl PermissionConfig {
    pub fn level_of(&self, name: &str, xuid: &str) -> PermissionLevel {
        self.players
            .iter()
            .find(|(player, _)| player.eq_ignore_ascii_case(name) || (!xuid.is_empty() && *player == xuid))
            .map_or(self.default, |(_, level)| *level)
    }
}

/// How many worker threads the task pools get: a number, or `"auto"` for one per CPU core.
#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(untagged)]
pub enum Threads {
    Count(usize),
    Mode(ThreadMode),
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "lowercase")]
pub enum ThreadMode {
    Auto,
}

impl Default for Threads {
    fn default() -> Self {
        Self::Mode(ThreadMode::Auto)
    }
}

impl Threads {
    pub fn count(self) -> usize {
        match self {
            Self::Count(count) => count,
            Self::Mode(ThreadMode::Auto) => std::thread::available_parallelism().map_or(1, |count| count.get()),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default, deny_unknown_fields)]
pub struct ServerConfig {
    pub name: String,
    /// Second line of the server list entry.
    pub description: String,
    pub max_players: i32,
    /// Whether players must be signed in to Xbox Live.
    pub authentication: bool,
    /// Worker threads for the task pools; 0 runs everything on the main thread.
    pub threads: Threads,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            name: String::from("Chorus"),
            description: String::from("bedrock-crustaceans.org"),
            max_players: 20,
            authentication: true,
            threads: Threads::default(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default, deny_unknown_fields)]
pub struct NetworkConfig {
    pub ip: String,
    pub port: u16,
    pub transport: NetworkTransport,
    pub raknet: RakNetConfig,
    pub nethernet: NetherNetConfig,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            ip: String::from("0.0.0.0"),
            port: 19132,
            transport: NetworkTransport::RakNet,
            raknet: RakNetConfig::default(),
            nethernet: NetherNetConfig::default(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default, deny_unknown_fields)]
pub struct RakNetConfig {
    pub encryption: bool,
}

impl Default for RakNetConfig {
    fn default() -> Self {
        Self { encryption: true }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default, deny_unknown_fields)]
pub struct NetherNetConfig {
    /// TCP port the HTTP signaling endpoint binds to.
    pub http_port: u16,
}

impl Default for NetherNetConfig {
    fn default() -> Self {
        Self { http_port: 19133 }
    }
}

/// How often something runs: a duration such as `"30s"`, `"5m"` or `"1h 30m"`, a whole number of
/// seconds, or `false` to turn it off.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Interval(Option<Duration>);

impl Interval {
    pub const DISABLED: Self = Self(None);

    pub const fn every(duration: Duration) -> Self {
        Self(Some(duration))
    }

    pub fn get(self) -> Option<Duration> {
        self.0.filter(|duration| !duration.is_zero())
    }
}

impl Serialize for Interval {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self.get() {
            Some(duration) => serializer.serialize_str(&humantime::format_duration(duration).to_string()),
            None => serializer.serialize_bool(false),
        }
    }
}

impl<'de> Deserialize<'de> for Interval {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Raw {
            Toggle(bool),
            Seconds(u64),
            Text(String),
        }
        match Raw::deserialize(deserializer)? {
            Raw::Toggle(false) => Ok(Self::DISABLED),
            Raw::Toggle(true) => Err(serde::de::Error::custom("expected a duration such as \"5m\" or false, not true")),
            Raw::Seconds(seconds) => Ok(Self::every(Duration::from_secs(seconds))),
            Raw::Text(text) => humantime::parse_duration(&text).map(Self::every).map_err(serde::de::Error::custom),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default, deny_unknown_fields)]
pub struct LevelConfig {
    pub name: String,
    pub seed: LevelSeed,
    pub max_view_distance: i32,
    /// How often unsaved chunks are written to disk.
    pub autosave: Interval,
    pub database: DatabaseConfig,
}

impl Default for LevelConfig {
    fn default() -> Self {
        Self {
            name: String::from("world"),
            seed: LevelSeed::default(),
            max_view_distance: 32,
            autosave: Interval::every(Duration::from_secs(60)),
            database: DatabaseConfig::default(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default, deny_unknown_fields)]
pub struct DatabaseConfig {
    /// Deflate level from 0 to 10.
    pub compression: u8,
    /// How often the database is compacted in the background.
    pub auto_compaction: Interval,
    /// Fully compact the database when the server stops, which can take a while after lots of new chunks.
    pub shutdown_compaction: bool,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            compression: 6,
            auto_compaction: Interval::every(Duration::from_secs(300)),
            shutdown_compaction: false,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default, deny_unknown_fields)]
pub struct PackConfig {
    pub resource_directory: PathBuf,
    pub behavior_directory: PathBuf,
    pub force_accept: bool,
    pub force_disable_vibrant_visuals: bool,
}

impl Default for PackConfig {
    fn default() -> Self {
        Self {
            resource_directory: PathBuf::from("resource_packs"),
            behavior_directory: PathBuf::from("behavior_packs"),
            force_accept: false,
            force_disable_vibrant_visuals: false,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default, deny_unknown_fields)]
pub struct LogConfig {
    pub level: String,
    pub to_file: bool,
    pub directory: PathBuf,
}

impl Default for LogConfig {
    fn default() -> Self {
        Self {
            level: String::from("info"),
            to_file: true,
            directory: PathBuf::from("logs"),
        }
    }
}

/// Where keys of older config layouts moved, as dotted (old path, new path).
const MOVED_KEYS: &[(&str, &str)] = &[
    ("name", "server.name"),
    ("sub_name", "server.description"),
    ("server.sub_name", "server.description"),
    ("max_players", "server.max_players"),
    ("online_mode", "server.authentication"),
    ("server.online_mode", "server.authentication"),
    ("threads", "server.threads"),
    ("ip", "network.ip"),
    ("port", "network.port"),
    ("transport", "network.transport"),
    ("nethernet_http_port", "network.nethernet.http_port"),
    ("network.nethernet_http_port", "network.nethernet.http_port"),
    ("encryption", "network.raknet.encryption"),
    ("network.encryption", "network.raknet.encryption"),
    ("level_name", "level.name"),
    ("level_seed", "level.seed"),
    ("level_compression_level", "level.database.compression"),
    ("level.compression_level", "level.database.compression"),
    ("max_view_distance", "level.max_view_distance"),
    ("resource_packs_directory", "packs.resource_directory"),
    ("behavior_packs_directory", "packs.behavior_directory"),
    ("force_accept_resource_packs", "packs.force_accept"),
    ("force_disable_vibrant_visuals", "packs.force_disable_vibrant_visuals"),
    ("log_level", "log.level"),
    ("log_to_file", "log.to_file"),
    ("logs_directory", "log.directory"),
];

/// Keys that no longer exist, as dotted paths.
const REMOVED_KEYS: &[&str] = &["max_generation_distance", "level.max_generation_distance"];

/// Removes the value at a dotted path, unless it is a section.
fn take_key(table: &mut toml::Table, path: &str) -> Option<toml::Value> {
    let (parents, key) = path.rsplit_once('.').map_or(("", path), |(parents, key)| (parents, key));
    let mut table = table;
    for part in parents.split('.').filter(|part| !part.is_empty()) {
        table = table.get_mut(part)?.as_table_mut()?;
    }
    if table.get(key)?.is_table() {
        return None;
    }
    table.remove(key)
}

/// Sets the value at a dotted path unless one is already there, creating sections on the way.
fn put_key(table: &mut toml::Table, path: &str, value: toml::Value) {
    let (parents, key) = path.rsplit_once('.').map_or(("", path), |(parents, key)| (parents, key));
    let mut table = table;
    for part in parents.split('.').filter(|part| !part.is_empty()) {
        let Some(section) = table.entry(part).or_insert_with(|| toml::Value::Table(toml::Table::new())).as_table_mut() else {
            return;
        };
        table = section;
    }
    table.entry(key).or_insert(value);
}

/// Moves keys of older config layouts to where they live now, drops removed keys and returns
/// whether anything changed.
fn migrate_layout(table: &mut toml::Table) -> bool {
    let mut migrated = false;
    for &path in REMOVED_KEYS {
        migrated |= take_key(table, path).is_some();
    }
    for &(old, new) in MOVED_KEYS {
        if let Some(value) = take_key(table, old) {
            put_key(table, new, value);
            migrated = true;
        }
    }
    migrated
}

impl Config {
    pub fn setup() -> Self {
        let config = if PathBuf::from(CONFIG_PATH).exists() {
            let text = fs::read_to_string(CONFIG_PATH).unwrap_or_else(|err| {
                eprintln!("An unexpected Error occurred while trying to read {CONFIG_PATH:?}, Err: {err}");
                exit(1);
            });

            let mut table: toml::Table = toml::from_str(&text).unwrap_or_else(|err| {
                eprintln!("An unexpected Error occurred while trying to parse {CONFIG_PATH:?}, Err: {err}");
                exit(1);
            });
            let migrated = migrate_layout(&mut table);
            let config: Config = table.try_into().unwrap_or_else(|err| {
                eprintln!("An unexpected Error occurred while trying to deserialize {CONFIG_PATH:?}, Err: {err}");
                exit(1);
            });
            if migrated {
                let backup = format!("{CONFIG_PATH}.old");
                match toml::to_string(&config) {
                    Ok(text) if fs::copy(CONFIG_PATH, &backup).is_ok() => match fs::write(CONFIG_PATH, text) {
                        Ok(()) => eprintln!("Moved {CONFIG_PATH:?} to the sectioned layout, the old file is kept as {backup:?}"),
                        Err(err) => eprintln!("Failed to write the migrated config to {CONFIG_PATH:?}, Err: {err}"),
                    },
                    _ => eprintln!("Failed to back up {CONFIG_PATH:?} before migrating it, using the migrated values without saving them"),
                }
            }
            config
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

        if !&config.log.directory.exists() {
            fs::create_dir(&config.log.directory).unwrap_or_else(|err| {
                eprintln!("An unexpected Error occurred while trying to create the logs directory at {:?}, Err: {err}", config.log.directory);
                exit(1)
            });
        };

        if !&config.packs.resource_directory.exists() {
            fs::create_dir(&config.packs.resource_directory).unwrap_or_else(|err| {
                eprintln!(
                    "An unexpected Error occurred while trying to create the resource packs directory at {:?}, Err: {err}",
                    config.packs.resource_directory
                );
                exit(1)
            });
        };

        if !&config.packs.behavior_directory.exists() {
            fs::create_dir(&config.packs.behavior_directory).unwrap_or_else(|err| {
                eprintln!(
                    "An unexpected Error occurred while trying to create the behavior packs directory at {:?}, Err: {err:?}",
                    config.packs.behavior_directory
                );
                exit(1)
            });
        };

        debug!("Config read!");

        config
    }
}
