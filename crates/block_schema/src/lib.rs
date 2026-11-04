use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PropertyValue {
    Byte(bool),
    Int(i32),
    String(String),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(from = "(PropertyValue, PropertyValue)", into = "(PropertyValue, PropertyValue)")]
pub struct ValueRemap {
    pub old: PropertyValue,
    pub new: PropertyValue,
}

impl From<(PropertyValue, PropertyValue)> for ValueRemap {
    fn from((old, new): (PropertyValue, PropertyValue)) -> Self {
        Self { old, new }
    }
}

impl From<ValueRemap> for (PropertyValue, PropertyValue) {
    fn from(remap: ValueRemap) -> Self {
        (remap.old, remap.new)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FlattenedPropertyType {
    #[default]
    String,
    Int,
    Byte,
}

impl FlattenedPropertyType {
    pub fn is_string(&self) -> bool {
        *self == Self::String
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FlattenRule {
    pub flattened_property: String,
    #[serde(default, skip_serializing_if = "FlattenedPropertyType::is_string")]
    pub flattened_property_type: FlattenedPropertyType,
    pub prefix: String,
    pub suffix: String,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub flattened_value_remaps: BTreeMap<String, String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct StateRemap {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub old_state: Option<BTreeMap<String, PropertyValue>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub new_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub new_flattened_name: Option<FlattenRule>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub new_state: Option<BTreeMap<String, PropertyValue>>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub copied_state: Vec<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Version {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
    pub revision: u32,
}

impl Version {
    pub fn packed(self) -> i32 {
        ((self.major << 24) | (self.minor << 16) | (self.patch << 8) | self.revision) as i32
    }
}

impl std::fmt::Display for Version {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}.{}.{}.{}", self.major, self.minor, self.patch, self.revision)
    }
}

impl Serialize for Version {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for Version {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let text = String::deserialize(deserializer)?;
        let parts: Vec<u32> = text.split('.').map(str::parse).collect::<Result<_, _>>().map_err(D::Error::custom)?;
        let [major, minor, patch, revision] = parts[..] else {
            return Err(D::Error::custom(format!("{text}: expected major.minor.patch.revision")));
        };
        Ok(Self { major, minor, patch, revision })
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Schema {
    pub version: Version,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub renamed_ids: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub added_properties: BTreeMap<String, BTreeMap<String, PropertyValue>>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub removed_properties: BTreeMap<String, Vec<String>>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub renamed_properties: BTreeMap<String, BTreeMap<String, String>>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub remapped_property_values: BTreeMap<String, BTreeMap<String, Vec<ValueRemap>>>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub flattened_properties: BTreeMap<String, FlattenRule>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub remapped_states: BTreeMap<String, Vec<StateRemap>>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BundledSchema {
    pub id: u32,
    pub name: String,
    #[serde(flatten)]
    pub schema: Schema,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Bundle {
    pub source: String,
    pub commit: String,
    pub schemas: Vec<BundledSchema>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_version_packs_into_the_number_blocks_carry() {
        let version = Version {
            major: 1,
            minor: 26,
            patch: 50,
            revision: 0,
        };
        assert_eq!(version.packed(), (1 << 24) | (26 << 16) | (50 << 8));
        assert_eq!(version.to_string(), "1.26.50.0");
    }

    #[test]
    fn versions_round_trip_through_text_and_reject_bad_text() {
        let version: Version = serde_json::from_str("\"1.21.0.25\"").expect("parses");
        assert_eq!(
            version,
            Version {
                major: 1,
                minor: 21,
                patch: 0,
                revision: 25
            }
        );
        assert_eq!(serde_json::to_string(&version).expect("serializes"), "\"1.21.0.25\"");
        assert!(serde_json::from_str::<Version>("\"1.21.0\"").is_err());
        assert!(serde_json::from_str::<Version>("\"one.two.three.four\"").is_err());
    }

    #[test]
    fn values_are_plain_json() {
        let values: Vec<PropertyValue> = serde_json::from_str("[true, 3, \"x\"]").expect("parses");
        assert_eq!(values, vec![PropertyValue::Byte(true), PropertyValue::Int(3), PropertyValue::String("x".into())]);
        let remap: ValueRemap = serde_json::from_str("[0, \"stone\"]").expect("parses");
        assert_eq!(
            remap,
            ValueRemap {
                old: PropertyValue::Int(0),
                new: PropertyValue::String("stone".into())
            }
        );
        assert_eq!(serde_json::to_string(&remap).expect("serializes"), "[0,\"stone\"]");
    }

    #[test]
    fn a_schema_leaves_out_what_is_empty_and_what_is_default() {
        let schema = Schema {
            version: Version {
                major: 1,
                minor: 19,
                patch: 60,
                revision: 0,
            },
            renamed_ids: BTreeMap::new(),
            added_properties: BTreeMap::new(),
            removed_properties: BTreeMap::new(),
            renamed_properties: BTreeMap::new(),
            remapped_property_values: BTreeMap::new(),
            flattened_properties: BTreeMap::from([(
                "minecraft:wool".to_owned(),
                FlattenRule {
                    flattened_property: "color".into(),
                    flattened_property_type: FlattenedPropertyType::String,
                    prefix: "minecraft:".into(),
                    suffix: "_wool".into(),
                    flattened_value_remaps: BTreeMap::new(),
                },
            )]),
            remapped_states: BTreeMap::new(),
        };
        let text = serde_json::to_string(&schema).expect("serializes");
        assert_eq!(
            text,
            r#"{"version":"1.19.60.0","flattened_properties":{"minecraft:wool":{"flattened_property":"color","prefix":"minecraft:","suffix":"_wool"}}}"#
        );
        assert_eq!(serde_json::from_str::<Schema>(&text).expect("parses"), schema);
    }
}
