use super::Config;
use std::fmt::{self, Display, Formatter};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub enum ConfigError {
    Read { path: PathBuf, source: io::Error },
    Parse { path: PathBuf, source: toml::de::Error },
    Section { name: String, source: toml::de::Error },
    UnknownKey { path: PathBuf, key: String },
}

impl Display for ConfigError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Read { path, source } => write!(f, "failed to read {}: {source}", path.display()),
            Self::Parse { path, source } => write!(f, "failed to parse {}: {source}", path.display()),
            Self::UnknownKey { path, key } => write!(f, "unknown key {key:?} in {}", path.display()),
            Self::Section { name, source } => write!(f, "invalid config section [{name}]: {source}"),
        }
    }
}

impl std::error::Error for ConfigError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Read { source, .. } => Some(source),
            Self::Parse { source, .. } | Self::Section { source, .. } => Some(source),
            Self::UnknownKey { .. } => None,
        }
    }
}

pub(super) fn read(path: &Path) -> Result<Config, ConfigError> {
    let text = fs::read_to_string(path).map_err(|source| ConfigError::Read { path: path.into(), source })?;
    let table: toml::Table = toml::from_str(&text).map_err(|source| ConfigError::Parse { path: path.into(), source })?;
    if let Some(key) = table.iter().find(|(_, value)| !value.is_table()).map(|(key, _)| key) {
        return Err(ConfigError::UnknownKey { path: path.into(), key: key.clone() });
    }

    Ok(Config::from_table(table))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ServerConfig;

    fn read_text(name: &str, text: &str) -> Result<Config, ConfigError> {
        let path = std::env::temp_dir().join(format!("chorus_config_{}_{name}.toml", std::process::id()));
        fs::write(&path, text).unwrap();
        let loaded = read(&path);
        fs::remove_file(&path).unwrap();
        loaded
    }

    #[test]
    fn a_top_level_key_that_is_not_a_section_is_rejected() {
        let error = read_text("typo", "nmae = \"x\"").err().unwrap();

        assert!(matches!(error, ConfigError::UnknownKey { ref key, .. } if key == "nmae"), "{error}");
    }

    #[test]
    fn other_sections_load_next_to_the_core_ones() {
        let config = read_text("sections", "[server]\nname = \"x\"\n[nethernet]\nsignaler = \"lan\"").unwrap();

        assert_eq!(config.get::<ServerConfig>().name, "x");
        assert_eq!(config.table["nethernet"]["signaler"].as_str(), Some("lan"));
    }
}
