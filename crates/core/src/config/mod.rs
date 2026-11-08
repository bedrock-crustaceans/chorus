mod app;
mod load;
mod sections;
mod values;

pub use app::{AppConfigExt, ConfigPlugin};
pub use load::ConfigError;
pub use sections::{DatabaseConfig, LevelConfig, LogConfig, NetworkChunksConfig, NetworkConfig, PackConfig, PermissionConfig, ServerConfig, SnappyConfig, ZLibConfig};
pub use values::{ChunkSaving, Interval, LevelSeed, NetworkCompression, ThreadMode, Threads};

use bevy_ecs::prelude::Resource;
use serde::Serialize;
use serde::de::DeserializeOwned;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::exit;

const CONFIG_PATH: &str = "chorus.toml";

pub trait ConfigSection: Resource + Serialize + DeserializeOwned + Default {
    const NAME: &'static str;
}

#[derive(Clone, Debug)]
struct Registered {
    name: &'static str,
    value: Option<toml::Value>,
}

#[derive(Resource, Clone, Debug, Default)]
pub struct Config {
    table: toml::Table,
    create_at: Option<PathBuf>,
    registered: Vec<Registered>,
}

impl Config {
    pub fn from_table(table: toml::Table) -> Self {
        Self { table, ..Self::default() }
    }

    pub fn with<T: ConfigSection>(mut self, section: T) -> Self {
        if let Ok(value) = toml::Value::try_from(&section) {
            self.table.insert(T::NAME.to_owned(), value);
        }
        self
    }

    pub fn get<T: ConfigSection>(&self) -> T {
        self.try_get().unwrap_or_else(|err| panic!("{err}"))
    }

    pub fn try_get<T: ConfigSection>(&self) -> Result<T, ConfigError> {
        match self.table.get(T::NAME) {
            None => Ok(T::default()),
            Some(raw) => raw.clone().try_into().map_err(|source| ConfigError::Section { name: T::NAME.to_owned(), source }),
        }
    }

    pub fn setup() -> Self {
        Self::open(Path::new(CONFIG_PATH)).unwrap_or_else(|err| {
            eprintln!("Failed to load the config: {err}");
            exit(1);
        })
    }

    pub fn open(path: &Path) -> Result<Self, ConfigError> {
        if path.exists() {
            load::read(path)
        } else {
            Ok(Self {
                create_at: Some(path.to_owned()),
                ..Self::default()
            })
        }
    }

    pub(crate) fn register<T: ConfigSection>(&mut self) -> Result<T, ConfigError> {
        let section = self.try_get::<T>()?;
        if !self.registered.iter().any(|registered| registered.name == T::NAME) {
            self.registered.push(Registered {
                name: T::NAME,
                value: toml::Value::try_from(&section).ok(),
            });
        }

        Ok(section)
    }

    pub(crate) fn create_missing(&self) {
        let Some(path) = &self.create_at else { return };
        let mut table = self.table.clone();
        for registered in &self.registered {
            if let Some(value) = &registered.value {
                table.insert(registered.name.to_owned(), value.clone());
            }
        }
        let Ok(text) = toml::to_string(&table) else { return };

        if let Ok(mut file) = OpenOptions::new().write(true).create_new(true).open(path) {
            let _ = file.write_all(text.as_bytes());
        }
    }

    pub(crate) fn unregistered(&self) -> impl Iterator<Item = &str> {
        self.table.keys().map(String::as_str).filter(|name| !self.registered.iter().any(|registered| registered.name == *name))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;

    #[derive(Resource, Serialize, Deserialize, Default, PartialEq, Debug)]
    #[serde(default, deny_unknown_fields)]
    struct Example {
        enabled: bool,
        limit: u32,
    }

    impl ConfigSection for Example {
        const NAME: &'static str = "example";
    }

    fn config(text: &str) -> Config {
        Config::from_table(toml::from_str(text).unwrap())
    }

    #[test]
    fn max_batch_size_defaults_when_the_config_omits_it() {
        let config = config("[network]\nport = 25565");

        assert_eq!(config.get::<NetworkConfig>().max_batch_size, NetworkConfig::default().max_batch_size);
        assert_eq!(config.get::<NetworkConfig>().port, 25565);
    }

    #[test]
    fn max_batch_size_is_read_from_the_config() {
        assert_eq!(config("[network]\nmax_batch_size = 1048576").get::<NetworkConfig>().max_batch_size, 1048576);
    }

    #[test]
    fn the_lookahead_defaults_to_none_and_is_read_from_the_config() {
        assert_eq!(Config::default().get::<LevelConfig>().lookahead, 0);
        assert_eq!(config("[level]\nlookahead = 4").get::<LevelConfig>().lookahead, 4);
    }

    #[test]
    fn the_retention_budget_defaults_to_one_view_and_is_read_from_the_config() {
        assert_eq!(Config::default().get::<LevelConfig>().retention_budget, 1.0);
        assert_eq!(config("[level]\nretention_budget = 0.5").get::<LevelConfig>().retention_budget, 0.5);
    }

    #[test]
    fn the_chunk_cache_defaults_to_half_a_view_and_is_read_from_the_config() {
        assert_eq!(Config::default().get::<LevelConfig>().chunk_cache, 0.5);
        assert_eq!(config("[level]\nchunk_cache = 0").get::<LevelConfig>().chunk_cache, 0.0);
    }

    #[test]
    fn a_section_is_read_and_a_missing_one_gives_the_defaults() {
        assert_eq!(config("[example]\nenabled = true\nlimit = 3").get::<Example>(), Example { enabled: true, limit: 3 });
        assert_eq!(Config::default().get::<Example>(), Example::default());
    }

    #[test]
    fn a_bad_section_names_the_section() {
        let error = config("[example]\nlimit = \"many\"").try_get::<Example>().unwrap_err();

        assert!(error.to_string().contains("[example]"), "{error}");
    }

    #[test]
    fn unknown_keys_in_a_section_are_rejected() {
        assert!(config("[server]\nnmae = \"typo\"").try_get::<ServerConfig>().is_err());
    }

    #[test]
    fn with_replaces_a_section() {
        let config = config("[example]\nlimit = 3").with(Example { enabled: true, limit: 9 });

        assert_eq!(config.get::<Example>(), Example { enabled: true, limit: 9 });
    }

    #[test]
    fn registering_a_section_claims_its_name() {
        let mut config = config("[example]\nlimit = 3\n[netherent]\nsignaler = \"lan\"");
        config.register::<Example>().unwrap();

        assert_eq!(config.unregistered().collect::<Vec<_>>(), ["netherent"]);
    }

    #[test]
    fn registering_twice_keeps_one_entry() {
        let mut config = Config::default();
        config.register::<Example>().unwrap();
        config.register::<Example>().unwrap();

        assert_eq!(config.registered.len(), 1);
    }

    #[test]
    fn a_bad_section_fails_to_register() {
        assert!(config("[example]\nlimit = \"many\"").register::<Example>().is_err());
    }

    fn temp_path(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("chorus_config_{}_{name}.toml", std::process::id()))
    }

    #[test]
    fn a_missing_config_is_created_with_the_registered_defaults_and_unknown_sections() {
        let path = temp_path("create");
        let _ = std::fs::remove_file(&path);
        let mut config = Config::open(&path).unwrap();
        config.register::<ServerConfig>().unwrap();
        config.register::<Example>().unwrap();
        config.table.insert("future".to_owned(), toml::Value::Table(toml::Table::new()));
        config.create_missing();

        let created: toml::Table = toml::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        std::fs::remove_file(&path).unwrap();

        assert!(created["server"].as_table().unwrap().contains_key("max_players"));
        assert!(created.contains_key("example") && created.contains_key("future"));
        assert!(!created.contains_key("level"));
    }

    #[test]
    fn an_existing_config_is_never_overwritten() {
        let path = temp_path("keep");
        std::fs::write(&path, "[server]\nname = \"mine\"\n").unwrap();
        let mut config = Config::open(&path).unwrap();
        config.register::<ServerConfig>().unwrap();
        config.create_missing();

        let text = std::fs::read_to_string(&path).unwrap();
        std::fs::remove_file(&path).unwrap();

        assert_eq!(text, "[server]\nname = \"mine\"\n");
    }

    #[test]
    fn a_config_built_in_code_never_writes() {
        let mut config = Config::default();
        config.register::<ServerConfig>().unwrap();
        config.create_missing();

        assert!(config.create_at.is_none());
    }
}
