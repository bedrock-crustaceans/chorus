use crate::core::config::ConfigSection;
use crate::level::Level;
use bevy_ecs::prelude::Resource;
use chorus_worldgen::{OverworldSettings, SettingsError, SettingsLayers};
use serde::{Deserialize, Serialize};

const GENERATOR_TAG: &str = "generator";

#[derive(Resource, Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[serde(try_from = "SettingsLayers", into = "SettingsLayers")]
pub struct WorldgenConfig {
    layers: SettingsLayers,
    settings: OverworldSettings,
}

impl TryFrom<SettingsLayers> for WorldgenConfig {
    type Error = SettingsError;

    fn try_from(layers: SettingsLayers) -> Result<Self, Self::Error> {
        let settings = layers.resolve()?;
        Ok(Self { layers, settings })
    }
}

impl From<WorldgenConfig> for SettingsLayers {
    fn from(config: WorldgenConfig) -> Self {
        config.layers
    }
}

impl ConfigSection for WorldgenConfig {
    const NAME: &'static str = "worldgen";
}

impl WorldgenConfig {
    pub fn layers(&self) -> &SettingsLayers {
        &self.layers
    }

    pub fn settings(&self) -> &OverworldSettings {
        &self.settings
    }
}

pub fn stored_settings(level: &Level) -> Option<Result<OverworldSettings, SettingsError>> {
    level.chorus_tag(GENERATOR_TAG).map(|tag| SettingsLayers::from_nbt(tag).and_then(|layers| layers.resolve()))
}

pub fn store_settings(level: &mut Level, layers: &SettingsLayers) {
    level.set_chorus_tag(GENERATOR_TAG, layers.to_nbt());
}

#[cfg(test)]
mod tests {
    use super::*;
    use chorus_worldgen::Preset;

    fn parse(text: &str) -> Result<WorldgenConfig, toml::de::Error> {
        toml::from_str(text)
    }

    #[test]
    fn the_config_layers_overrides_on_a_preset() {
        let config = parse("preset = \"large_biomes\"\n[climate]\ntemperature = 6.0\n[terrain]\nglobal_offset = -0.4").unwrap();
        let settings = config.settings();
        assert_eq!(settings.climate.temperature, 6.0);
        assert_eq!(settings.climate.erosion, 4.0);
        assert_eq!(settings.terrain.global_offset, -0.4);
    }

    #[test]
    fn the_config_reads_every_kind_of_setting() {
        let text = "preset = \"amplified\"\nstructures = false\nbiome_source = { checkerboard = { biomes = [\"plains\", \"desert\"], scale = 3 } }\n[terrain]\nsea_level = 40\naquifers = false\nore_veins = false";
        let settings = parse(text).unwrap().settings().clone();
        assert_eq!(settings.terrain.sea_level, 40);
        assert!(!settings.terrain.aquifers && !settings.terrain.ore_veins && !settings.structures);
        assert_eq!(settings.terrain.offset, Preset::Amplified.settings().terrain.offset);
        assert!(matches!(settings.biome_source, chorus_worldgen::BiomeSource::Checkerboard(ref checkerboard) if checkerboard.scale == 3));
    }

    #[test]
    fn a_bad_section_fails_to_parse() {
        assert!(parse("[climat]\ntemperature = 6.0").is_err());
        assert!(parse("[terrain]\nsea_level = 500").is_err());
        assert!(parse("preset = \"flat\"").is_err());
    }

    #[test]
    fn the_default_section_can_be_written_and_read_back() {
        let written = toml::Value::try_from(WorldgenConfig::default()).unwrap();
        let read: WorldgenConfig = written.try_into().unwrap();
        assert_eq!(read.settings(), &OverworldSettings::default());
    }

    #[test]
    fn the_default_config_is_vanilla() {
        assert_eq!(WorldgenConfig::default().settings(), &OverworldSettings::default());
    }

    #[test]
    fn the_world_stores_the_configured_layers_as_nbt() {
        let mut level = Level::in_memory("stored", 0);
        assert!(stored_settings(&level).is_none());
        let config = parse(
            "preset = \"amplified\"
[terrain]
sea_level = 40",
        )
        .unwrap();
        store_settings(&mut level, config.layers());
        assert_eq!(stored_settings(&level), Some(Ok(config.settings().clone())));
        let Some(nbtx::Value::Compound(generator)) = level.chorus_tag("generator") else {
            panic!("a compound tag")
        };
        assert!(
            generator
                .iter()
                .any(|(key, value)| key.as_slice() == b"preset" && matches!(value, nbtx::Value::String(name) if name == "amplified"))
        );
    }

    #[test]
    fn a_hand_written_tag_may_name_only_a_preset() {
        let mut level = Level::in_memory("named", 0);
        level.set_chorus_tag(
            "generator",
            nbtx::Value::Compound(nbtx::Compound::from_iter([("preset".into(), nbtx::Value::String("large_biomes".into()))])),
        );
        assert_eq!(stored_settings(&level), Some(Ok(Preset::LargeBiomes.settings())));
    }

    #[test]
    fn a_broken_tag_is_reported_not_ignored() {
        let mut level = Level::in_memory("broken", 0);
        level.set_chorus_tag("generator", nbtx::Value::Int(5));
        assert!(matches!(stored_settings(&level), Some(Err(_))));
    }
}
