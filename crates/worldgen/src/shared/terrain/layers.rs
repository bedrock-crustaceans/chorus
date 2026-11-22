use super::settings::{OverworldSettings, Preset, SettingsError};
use nbtx::{Compound, Value as Nbt, ValueList};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Number, Value};

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct SettingsLayers {
    pub preset: Preset,
    #[serde(flatten)]
    pub overrides: Map<String, Value>,
}

impl SettingsLayers {
    pub fn resolve(&self) -> Result<OverworldSettings, SettingsError> {
        OverworldSettings::resolve(self.preset, &self.overrides)
    }

    pub fn to_nbt(&self) -> Nbt {
        let json = serde_json::to_value(self).expect("settings layers are plain data");
        to_nbt(&json).unwrap_or_else(|| Nbt::Compound(Compound::new()))
    }

    pub fn from_nbt(tag: &Nbt) -> Result<Self, SettingsError> {
        if !matches!(tag, Nbt::Compound(_)) {
            return Err(SettingsError::Malformed("the generator tag is not a compound".into()));
        }
        serde_json::from_value(from_nbt(tag)?).map_err(|err| SettingsError::Malformed(err.to_string()))
    }
}

fn to_nbt(value: &Value) -> Option<Nbt> {
    Some(match value {
        Value::Null => return None,
        Value::Bool(flag) => Nbt::Byte(i8::from(*flag)),
        Value::Number(number) => number_to_nbt(number),
        Value::String(text) => Nbt::String(text.as_str().into()),
        Value::Array(items) => {
            let values: Vec<Nbt> = items.iter().filter_map(to_nbt).collect();
            Nbt::List(ValueList::try_from(values).unwrap_or_default())
        }
        Value::Object(fields) => Nbt::Compound(fields.iter().filter_map(|(key, field)| to_nbt(field).map(|tag| (key.as_str().into(), tag))).collect()),
    })
}

fn number_to_nbt(number: &Number) -> Nbt {
    match number.as_i64() {
        Some(integer) => i32::try_from(integer).map_or(Nbt::Long(integer), Nbt::Int),
        None => Nbt::Float(number.as_f64().unwrap_or_default() as f32),
    }
}

fn from_nbt(tag: &Nbt) -> Result<Value, SettingsError> {
    let malformed = |what: &str| SettingsError::Malformed(format!("unsupported tag in the generator settings: {what}"));
    Ok(match tag {
        Nbt::Byte(flag) => Value::Bool(*flag != 0),
        Nbt::Short(integer) => Value::from(*integer),
        Nbt::Int(integer) => Value::from(*integer),
        Nbt::Long(integer) => Value::from(*integer),
        Nbt::Float(float) => Number::from_f64(f64::from(*float)).map(Value::Number).ok_or_else(|| malformed("a non-finite float"))?,
        Nbt::Double(float) => Number::from_f64(*float).map(Value::Number).ok_or_else(|| malformed("a non-finite double"))?,
        Nbt::String(text) => Value::String(String::from_utf8(text.to_vec()).map_err(|_| malformed("a string that is not UTF-8"))?),
        Nbt::List(list) => Value::Array(list.to_values().iter().map(from_nbt).collect::<Result<_, _>>()?),
        Nbt::Compound(fields) => {
            let mut object = Map::new();
            for (key, field) in fields {
                let key = String::from_utf8(key.to_vec()).map_err(|_| malformed("a key that is not UTF-8"))?;
                object.insert(key, from_nbt(field)?);
            }
            Value::Object(object)
        }
        Nbt::ByteArray(_) | Nbt::IntArray(_) | Nbt::LongArray(_) => return Err(malformed("an array tag")),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn field<'a>(compound: &'a Compound, name: &str) -> Option<&'a Nbt> {
        compound.iter().find(|(key, _)| key.as_slice() == name.as_bytes()).map(|(_, value)| value)
    }

    fn layers(preset: Preset, overrides: &str) -> SettingsLayers {
        SettingsLayers {
            preset,
            overrides: serde_json::from_str(overrides).unwrap(),
        }
    }

    #[test]
    fn layers_survive_a_round_trip_through_nbt() {
        let cases = [
            layers(Preset::Default, "{}"),
            layers(
                Preset::Amplified,
                r#"{"terrain": {"sea_level": 40, "aquifers": false, "offset": {"scale": {"below_zero": 1.0, "above_zero": 3.5}}}}"#,
            ),
            layers(
                Preset::LargeBiomes,
                r#"{"climate": {"temperature": 8.0}, "structures": false, "biome_source": {"checkerboard": {"biomes": ["plains", "desert"], "scale": 3}}}"#,
            ),
            layers(Preset::Default, r#"{"biome_source": {"fixed": "minecraft:plains"}}"#),
        ];
        for case in cases {
            let tag = case.to_nbt();
            assert_eq!(SettingsLayers::from_nbt(&tag).unwrap().resolve(), case.resolve(), "{case:?}");
        }
    }

    #[test]
    fn the_tag_uses_real_nbt_types() {
        let tag = layers(
            Preset::Amplified,
            r#"{"terrain": {"sea_level": 40, "aquifers": false, "global_offset": -0.25}, "biome_source": {"fixed": "plains"}}"#,
        )
        .to_nbt();
        let Nbt::Compound(root) = &tag else { panic!("a compound") };
        assert!(matches!(field(root, "preset"), Some(Nbt::String(name)) if name == "amplified"));
        let Some(Nbt::Compound(terrain)) = field(root, "terrain") else { panic!("terrain") };
        assert!(matches!(field(terrain, "sea_level"), Some(Nbt::Int(40))));
        assert!(matches!(field(terrain, "aquifers"), Some(Nbt::Byte(0))));
        assert!(matches!(field(terrain, "global_offset"), Some(Nbt::Float(value)) if *value == -0.25));
    }

    #[test]
    fn a_preset_name_alone_is_enough() {
        let mut root = Compound::new();
        root.insert("preset".into(), Nbt::String("large_biomes".into()));
        let layers = SettingsLayers::from_nbt(&Nbt::Compound(root)).unwrap();
        assert_eq!(layers.resolve(), Ok(Preset::LargeBiomes.settings()));
    }

    #[test]
    fn a_table_without_a_preset_overrides_the_defaults() {
        let mut terrain = Compound::new();
        terrain.insert("sea_level".into(), Nbt::Int(20));
        let mut root = Compound::new();
        root.insert("terrain".into(), Nbt::Compound(terrain));
        let settings = SettingsLayers::from_nbt(&Nbt::Compound(root)).unwrap().resolve().unwrap();
        assert_eq!(settings.terrain.sea_level, 20);
        assert_eq!(settings.climate, OverworldSettings::default().climate);
    }

    #[test]
    fn bad_tags_are_reported() {
        assert!(SettingsLayers::from_nbt(&Nbt::Int(3)).is_err());
        let mut root = Compound::new();
        root.insert("climat".into(), Nbt::Compound(Compound::new()));
        assert!(SettingsLayers::from_nbt(&Nbt::Compound(root)).unwrap().resolve().is_err());
    }
}
