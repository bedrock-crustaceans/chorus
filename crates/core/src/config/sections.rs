use super::ConfigSection;
use super::values::{ChunkSaving, Interval, LevelSeed, NetworkCompression, Threads};
use crate::permission::PermissionLevel;
use bevy_ecs::prelude::Resource;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;

#[derive(Resource, Serialize, Deserialize, Clone, Debug)]
#[serde(default, deny_unknown_fields)]
pub struct PermissionConfig {
    pub default: PermissionLevel,
    pub players: std::collections::BTreeMap<String, PermissionLevel>,
}

impl Default for PermissionConfig {
    fn default() -> Self {
        Self {
            default: PermissionLevel::Member,
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

#[derive(Resource, Serialize, Deserialize, Clone, Debug)]
#[serde(default, deny_unknown_fields)]
pub struct ServerConfig {
    pub name: String,
    pub description: String,
    pub max_players: i32,
    pub authentication: bool,
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

#[derive(Resource, Serialize, Deserialize, Clone, Debug)]
#[serde(default, deny_unknown_fields)]
pub struct NetworkConfig {
    pub ip: String,
    pub port: u16,
    pub max_batch_size: usize,
    pub compression: NetworkCompression,
    pub zlib: ZLibConfig,
    pub snappy: SnappyConfig,
    pub chunks: NetworkChunksConfig,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            ip: String::from("0.0.0.0"),
            port: 19132,
            max_batch_size: 16 * 1024 * 1024,
            compression: NetworkCompression::Zlib,
            zlib: ZLibConfig::default(),
            snappy: SnappyConfig::default(),
            chunks: NetworkChunksConfig::default(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default, deny_unknown_fields)]
pub struct ZLibConfig {
    pub threshold: u16,
    pub level: u8,
}

impl Default for ZLibConfig {
    fn default() -> Self {
        Self { threshold: 1, level: 6 }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default, deny_unknown_fields)]
pub struct SnappyConfig {
    pub threshold: u16,
}

impl Default for SnappyConfig {
    fn default() -> Self {
        Self { threshold: 1 }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(default, deny_unknown_fields)]
pub struct NetworkChunksConfig {
    // TODO: subchunks disabled by default, since we don't have their heightmaps implemented yet
    pub subchunks: bool,
    pub cache: bool,
}

#[derive(Resource, Serialize, Deserialize, Clone, Debug)]
#[serde(default, deny_unknown_fields)]
pub struct LevelConfig {
    pub name: String,
    pub seed: LevelSeed,
    pub chunk_saving: ChunkSaving,
    pub max_view_distance: i32,
    /// How many generated pieces of each kind to keep per player for neighbouring chunks, as a multiple of the chunks in the player's view.
    /// A lower value uses less memory and regenerates more, 0 keeps nothing.
    pub retention_budget: f64,
    /// How many chunks beyond the loaded area a player is assumed to be able to walk before the generator has to be ready, which is how far outside
    /// the view generated pieces are kept for. A higher value uses more memory and regenerates less for a player on the move.
    pub lookahead: i32,
    /// How many chunks to keep after they leave every player's view, as a multiple of the chunks in a view. A player who walks back to them does not wait for
    /// them to generate again. 0 drops them at once.
    pub chunk_cache: f64,
    pub autosave: Interval,
    pub database: DatabaseConfig,
}

impl Default for LevelConfig {
    fn default() -> Self {
        Self {
            name: String::from("world"),
            seed: LevelSeed::default(),
            chunk_saving: ChunkSaving::default(),
            max_view_distance: 32,
            retention_budget: 1.0,
            lookahead: 0,
            chunk_cache: 0.5,
            autosave: Interval::every(Duration::from_secs(60)),
            database: DatabaseConfig::default(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default, deny_unknown_fields)]
pub struct DatabaseConfig {
    pub compression: u8,
    pub auto_compaction: Interval,
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

#[derive(Resource, Serialize, Deserialize, Clone, Debug)]
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

#[derive(Resource, Serialize, Deserialize, Clone, Debug)]
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

impl ConfigSection for ServerConfig {
    const NAME: &'static str = "server";
}

impl ConfigSection for NetworkConfig {
    const NAME: &'static str = "network";
}

impl ConfigSection for LevelConfig {
    const NAME: &'static str = "level";
}

impl ConfigSection for PackConfig {
    const NAME: &'static str = "packs";
}

impl ConfigSection for LogConfig {
    const NAME: &'static str = "log";
}

impl ConfigSection for PermissionConfig {
    const NAME: &'static str = "permissions";
}
