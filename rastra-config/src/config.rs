use serde_yaml::Value;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub enum ConfigType {
    Yaml,
    Properties,
    Json,
    Toml,
}

pub struct Config {
    path: PathBuf,
    config_type: ConfigType,
    data: Value,
}

impl Config {
    pub fn new(path: impl AsRef<Path>, config_type: ConfigType) -> Self {
        let path = path.as_ref().to_path_buf();

        if !path.exists() {
            fs::write(&path, "").unwrap();
        }

        let content = fs::read_to_string(&path).unwrap_or_default();

        let data = match config_type {
            ConfigType::Yaml => {
                serde_yaml::from_str(&content).unwrap_or(Value::Mapping(Default::default()))
            }
            ConfigType::Properties => {
                let props: HashMap<String, String> =
                    java_properties::read(content.as_bytes()).unwrap_or_default();

                serde_yaml::to_value(props).unwrap()
            }
            ConfigType::Json => {
                serde_json::from_str(&content).unwrap_or(Value::Mapping(Default::default()))
            }
            ConfigType::Toml => {
                let value: toml::Value = toml::from_str(&content).unwrap_or(toml::Value::Table(Default::default()));
                serde_yaml::to_value(value).unwrap()
            }
        };

        Self {
            path,
            config_type,
            data,
        }
    }

    pub fn get_string(&self, key: &str) -> Option<String> {
        self.get(key).and_then(|v| v.as_str().map(|s| s.to_string()))
    }

    pub fn get_int(&self, key: &str) -> Option<i64> {
        self.get(key).and_then(|v| v.as_i64())
    }

    pub fn get_bool(&self, key: &str) -> Option<bool> {
        self.get(key).and_then(|v| v.as_bool())
    }

    pub fn get(&self, key: &str) -> Option<&Value> {
        let mut current = &self.data;

        for part in key.split('.') {
            current = current.get(part)?;
        }

        Some(current)
    }

    pub fn set(&mut self, key: &str, value: Value) {
        let mut current = &mut self.data;

        let parts: Vec<&str> = key.split('.').collect();

        for part in &parts[..parts.len() - 1] {
            current = current
                .as_mapping_mut()
                .unwrap()
                .entry(Value::String(part.to_string()))
                .or_insert(Value::Mapping(Default::default()));
        }

        current
            .as_mapping_mut()
            .unwrap()
            .insert(Value::String(parts.last().unwrap().to_string()), value);
    }

    pub fn save(&self) {
        match self.config_type {
            ConfigType::Yaml => {
                let content = serde_yaml::to_string(&self.data).unwrap();
                fs::write(&self.path, content).unwrap();
            }
            ConfigType::Properties => {
                let map: HashMap<String, String> =
                    serde_yaml::from_value(self.data.clone()).unwrap_or_default();

                let mut buf = Vec::new();
                java_properties::write(&mut buf, &map).unwrap();
                fs::write(&self.path, buf).unwrap();
            }
            ConfigType::Json => {
                let content = serde_json::to_string_pretty(&self.data).unwrap();
                fs::write(&self.path, content).unwrap();
            }
            ConfigType::Toml => {
                let value: toml::Value = serde_yaml::from_value(self.data.clone()).unwrap();
                let content = toml::to_string_pretty(&value).unwrap();
                fs::write(&self.path, content).unwrap();
            }
        }
    }

    pub fn reload(&mut self) {
        let content = fs::read_to_string(&self.path).unwrap_or_default();

        self.data = serde_yaml::from_str(&content)
            .unwrap_or(Value::Mapping(Default::default()));
    }
}