use crate::shared::SEA_LEVEL;
use crate::shared::biome::Biome;
use crate::shared::math::spline::LeafTransform;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::fmt;

pub const VANILLA_GLOBAL_OFFSET: f32 = -0.50375;
pub const VANILLA_FACTOR_SCALER: f32 = 128.0;
pub const LOWEST_SEA_LEVEL: i32 = -54;
pub const HIGHEST_SEA_LEVEL: i32 = 319;
pub const HIGHEST_CHECKERBOARD_SCALE: u8 = 20;
const DEFAULT_CHECKERBOARD_SCALE: u8 = 2;
const CHECKERBOARD_QUART_SHIFT: u32 = 2;
const MERGED_SECTIONS: [&str; 2] = ["climate", "terrain"];

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ClimateScale {
    pub temperature: f32,
    pub humidity: f32,
    pub continentalness: f32,
    pub erosion: f32,
    pub weirdness: f32,
}

impl Default for ClimateScale {
    fn default() -> Self {
        Self::uniform(1.0)
    }
}

impl ClimateScale {
    pub const fn uniform(scale: f32) -> Self {
        Self {
            temperature: scale,
            humidity: scale,
            continentalness: scale,
            erosion: scale,
            weirdness: scale,
        }
    }

    fn all(self) -> [(&'static str, f32); 5] {
        [
            ("temperature", self.temperature),
            ("humidity", self.humidity),
            ("continentalness", self.continentalness),
            ("erosion", self.erosion),
            ("weirdness", self.weirdness),
        ]
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct TerrainShape {
    pub offset: LeafTransform,
    pub factor: LeafTransform,
    pub jaggedness: LeafTransform,
    pub global_offset: f32,
    pub factor_scaler: f32,
    pub sea_level: i32,
    pub aquifers: bool,
    pub ore_veins: bool,
}

impl Default for TerrainShape {
    fn default() -> Self {
        Self {
            offset: LeafTransform::Identity,
            factor: LeafTransform::Identity,
            jaggedness: LeafTransform::Identity,
            global_offset: VANILLA_GLOBAL_OFFSET,
            factor_scaler: VANILLA_FACTOR_SCALER,
            sea_level: SEA_LEVEL,
            aquifers: true,
            ore_veins: true,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Checkerboard {
    pub biomes: Vec<Biome>,
    #[serde(default = "default_checkerboard_scale")]
    pub scale: u8,
}

fn default_checkerboard_scale() -> u8 {
    DEFAULT_CHECKERBOARD_SCALE
}

impl Checkerboard {
    pub fn at(&self, x: i32, z: i32) -> Biome {
        let shift = self.scale as u32 + CHECKERBOARD_QUART_SHIFT;
        let square = ((x >> 2) >> shift) + ((z >> 2) >> shift);
        self.biomes[square.rem_euclid(self.biomes.len() as i32) as usize].clone()
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BiomeSource {
    #[default]
    MultiNoise,
    Fixed(Biome),
    Checkerboard(Checkerboard),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct OverworldSettings {
    pub climate: ClimateScale,
    pub terrain: TerrainShape,
    pub biome_source: BiomeSource,
    pub structures: bool,
}

impl Default for OverworldSettings {
    fn default() -> Self {
        Self {
            climate: ClimateScale::default(),
            terrain: TerrainShape::default(),
            biome_source: BiomeSource::default(),
            structures: true,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Preset {
    #[default]
    Default,
    LargeBiomes,
    Amplified,
    AmplifiedLargeBiomes,
}

impl Preset {
    pub fn settings(self) -> OverworldSettings {
        let amplified = TerrainShape {
            offset: LeafTransform::AMPLIFIED_OFFSET,
            factor: LeafTransform::AMPLIFIED_FACTOR,
            jaggedness: LeafTransform::AMPLIFIED_JAGGEDNESS,
            ..TerrainShape::default()
        };
        let large = ClimateScale {
            temperature: 4.0,
            humidity: 4.0,
            continentalness: 4.0,
            erosion: 4.0,
            weirdness: 1.0,
        };
        let vanilla = OverworldSettings::default();
        match self {
            Self::Default => vanilla,
            Self::LargeBiomes => OverworldSettings { climate: large, ..vanilla },
            Self::Amplified => OverworldSettings { terrain: amplified, ..vanilla },
            Self::AmplifiedLargeBiomes => OverworldSettings {
                climate: large,
                terrain: amplified,
                ..vanilla
            },
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum SettingsError {
    Invalid(String),
    Malformed(String),
    Climate(String),
}

impl fmt::Display for SettingsError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Invalid(reason) => write!(formatter, "invalid overworld settings: {reason}"),
            Self::Malformed(reason) => write!(formatter, "malformed overworld settings: {reason}"),
            Self::Climate(reason) => write!(formatter, "{reason}"),
        }
    }
}

impl std::error::Error for SettingsError {}

impl OverworldSettings {
    pub fn single_biome(biome: Biome) -> Self {
        Self {
            biome_source: BiomeSource::Fixed(biome),
            ..Self::default()
        }
    }

    pub fn validate(&self) -> Result<(), SettingsError> {
        for (name, scale) in self.climate.all() {
            if !scale.is_finite() || scale <= 0.0 {
                return Err(SettingsError::Invalid(format!("climate.{name} must be a positive number, got {scale}")));
            }
        }
        let terrain = &self.terrain;
        for (name, transform) in [("offset", terrain.offset), ("factor", terrain.factor), ("jaggedness", terrain.jaggedness)] {
            if !transform.is_finite() {
                return Err(SettingsError::Invalid(format!("terrain.{name} must use finite numbers")));
            }
        }
        for (name, value) in [("global_offset", terrain.global_offset), ("factor_scaler", terrain.factor_scaler)] {
            if !value.is_finite() {
                return Err(SettingsError::Invalid(format!("terrain.{name} must be a finite number, got {value}")));
            }
        }
        if !(LOWEST_SEA_LEVEL..=HIGHEST_SEA_LEVEL).contains(&terrain.sea_level) {
            return Err(SettingsError::Invalid(format!(
                "terrain.sea_level must be between {LOWEST_SEA_LEVEL} and {HIGHEST_SEA_LEVEL}, got {}",
                terrain.sea_level
            )));
        }
        if let BiomeSource::Checkerboard(checkerboard) = &self.biome_source {
            if checkerboard.biomes.is_empty() {
                return Err(SettingsError::Invalid("biome_source.checkerboard needs at least one biome".into()));
            }
            if checkerboard.scale > HIGHEST_CHECKERBOARD_SCALE {
                return Err(SettingsError::Invalid(format!(
                    "biome_source.checkerboard.scale must be at most {HIGHEST_CHECKERBOARD_SCALE}, got {}",
                    checkerboard.scale
                )));
            }
        }
        Ok(())
    }

    pub fn resolve(preset: Preset, overrides: &Map<String, Value>) -> Result<Self, SettingsError> {
        let Value::Object(mut merged) = serde_json::to_value(preset.settings()).expect("settings are plain data") else {
            unreachable!("settings serialize to an object");
        };
        merge(&mut merged, overrides);
        Self::from_value(Value::Object(merged))
    }

    fn from_value(value: Value) -> Result<Self, SettingsError> {
        let settings: Self = serde_json::from_value(value).map_err(|err| SettingsError::Malformed(err.to_string()))?;
        settings.validate()?;
        Ok(settings)
    }
}

fn merge(base: &mut Map<String, Value>, overrides: &Map<String, Value>) {
    for (key, value) in overrides {
        match (base.get_mut(key), value) {
            (Some(Value::Object(section)), Value::Object(fields)) if MERGED_SECTIONS.contains(&key.as_str()) => section.extend(fields.clone()),
            _ => {
                base.insert(key.clone(), value.clone());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn overrides(json: &str) -> Map<String, Value> {
        serde_json::from_str(json).unwrap()
    }

    fn rejected(json: &str) -> bool {
        OverworldSettings::resolve(Preset::Default, &overrides(json)).is_err()
    }

    #[test]
    fn settings_survive_a_serde_round_trip() {
        let checkerboard = OverworldSettings {
            biome_source: BiomeSource::Checkerboard(Checkerboard {
                biomes: vec![Biome::PLAINS, Biome::DESERT],
                scale: 3,
            }),
            ..OverworldSettings::default()
        };
        let every = [Preset::Default, Preset::LargeBiomes, Preset::Amplified, Preset::AmplifiedLargeBiomes].map(Preset::settings);
        for settings in every.into_iter().chain([OverworldSettings::single_biome(Biome::DESERT), checkerboard]) {
            assert_eq!(serde_json::from_value::<OverworldSettings>(serde_json::to_value(&settings).unwrap()).unwrap(), settings);
        }
    }

    #[test]
    fn overrides_replace_single_fields_of_a_preset() {
        let settings = OverworldSettings::resolve(
            Preset::LargeBiomes,
            &overrides(r#"{"climate": {"temperature": 8.0}, "terrain": {"sea_level": 40}, "structures": false}"#),
        )
        .unwrap();
        assert_eq!(settings.climate.temperature, 8.0);
        assert_eq!(settings.climate.erosion, 4.0);
        assert_eq!(settings.terrain.sea_level, 40);
        assert!(!settings.structures);
    }

    #[test]
    fn a_biome_source_override_replaces_the_whole_source() {
        let fixed = OverworldSettings::resolve(Preset::Default, &overrides(r#"{"biome_source": {"fixed": "minecraft:plains"}}"#)).unwrap();
        assert_eq!(fixed.biome_source, BiomeSource::Fixed(Biome::PLAINS));
        let checkerboard = r#"{"checkerboard": {"biomes": ["plains", "desert"]}}"#;
        let layered = OverworldSettings::resolve(Preset::Default, &overrides(&format!(r#"{{"biome_source": {checkerboard}}}"#))).unwrap();
        assert_eq!(
            layered.biome_source,
            BiomeSource::Checkerboard(Checkerboard {
                biomes: vec![Biome::PLAINS, Biome::DESERT],
                scale: 2
            })
        );
    }

    #[test]
    fn unknown_fields_and_biomes_are_errors() {
        assert!(rejected(r#"{"climat": {}}"#));
        assert!(rejected(r#"{"terrain": {"sealevel": 40}}"#));
        assert!(rejected(r#"{"biome_source": {"fixed": "minecraft:nowhere"}}"#));
    }

    #[test]
    fn values_must_be_in_range() {
        assert!(rejected(r#"{"climate": {"erosion": 0.0}}"#));
        assert!(rejected(r#"{"climate": {"erosion": -2.0}}"#));
        assert!(rejected(r#"{"terrain": {"global_offset": 1e60}}"#));
        assert!(rejected(r#"{"terrain": {"sea_level": -55}}"#));
        assert!(rejected(r#"{"terrain": {"sea_level": 320}}"#));
        assert!(rejected(r#"{"biome_source": {"checkerboard": {"biomes": []}}}"#));
        assert!(rejected(r#"{"biome_source": {"checkerboard": {"biomes": ["plains"], "scale": 21}}}"#));
    }
}
