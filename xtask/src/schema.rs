use chorus_block_schema::{FlattenRule, FlattenedPropertyType, PropertyValue, Schema, StateRemap, ValueRemap, Version};
use serde::Deserialize;
use std::collections::BTreeMap;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
enum RawValue {
    String(String),
    Int(i32),
    Byte(i8),
}

#[derive(Deserialize)]
struct RawValueRemap {
    old: RawValue,
    new: RawValue,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct RawFlattenRule {
    flattened_property: String,
    #[serde(default)]
    flattened_property_type: FlattenedPropertyType,
    prefix: String,
    suffix: String,
    #[serde(default)]
    flattened_value_remaps: BTreeMap<String, String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct RawStateRemap {
    old_state: Option<BTreeMap<String, RawValue>>,
    new_name: Option<String>,
    new_flattened_name: Option<RawFlattenRule>,
    new_state: Option<BTreeMap<String, RawValue>>,
    #[serde(default)]
    copied_state: Vec<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RawSchema {
    max_version_major: u32,
    max_version_minor: u32,
    max_version_patch: u32,
    max_version_revision: u32,
    #[serde(default)]
    renamed_ids: BTreeMap<String, String>,
    #[serde(default)]
    added_properties: BTreeMap<String, BTreeMap<String, RawValue>>,
    #[serde(default)]
    removed_properties: BTreeMap<String, Vec<String>>,
    #[serde(default)]
    renamed_properties: BTreeMap<String, BTreeMap<String, String>>,
    #[serde(default)]
    remapped_property_values: BTreeMap<String, BTreeMap<String, String>>,
    #[serde(default)]
    remapped_property_values_index: BTreeMap<String, Vec<RawValueRemap>>,
    #[serde(default)]
    flattened_properties: BTreeMap<String, RawFlattenRule>,
    #[serde(default)]
    remapped_states: BTreeMap<String, Vec<RawStateRemap>>,
}

fn value(raw: &RawValue) -> PropertyValue {
    match raw {
        RawValue::String(string) => PropertyValue::String(string.clone()),
        RawValue::Int(int) => PropertyValue::Int(*int),
        RawValue::Byte(0) => PropertyValue::Byte(false),
        RawValue::Byte(1) => PropertyValue::Byte(true),
        RawValue::Byte(other) => panic!("byte property value {other} is neither 0 nor 1"),
    }
}

fn values(raw: &BTreeMap<String, RawValue>) -> BTreeMap<String, PropertyValue> {
    raw.iter().map(|(name, raw)| (name.clone(), value(raw))).collect()
}

fn flatten_rule(raw: RawFlattenRule) -> FlattenRule {
    FlattenRule {
        flattened_property: raw.flattened_property,
        flattened_property_type: raw.flattened_property_type,
        prefix: raw.prefix,
        suffix: raw.suffix,
        flattened_value_remaps: raw.flattened_value_remaps,
    }
}

fn state_remap(raw: RawStateRemap) -> StateRemap {
    StateRemap {
        old_state: raw.old_state.as_ref().map(values),
        new_name: raw.new_name,
        new_flattened_name: raw.new_flattened_name.map(flatten_rule),
        new_state: raw.new_state.as_ref().map(values),
        copied_state: raw.copied_state,
    }
}

fn inline_value_remaps(remapped: BTreeMap<String, BTreeMap<String, String>>, index: &BTreeMap<String, Vec<RawValueRemap>>) -> BTreeMap<String, BTreeMap<String, Vec<ValueRemap>>> {
    remapped
        .into_iter()
        .map(|(block, properties)| {
            let properties = properties
                .into_iter()
                .map(|(property, name)| {
                    let remaps = index.get(&name).unwrap_or_else(|| panic!("{block}.{property}: value remap {name} is not in the index"));
                    (
                        property,
                        remaps
                            .iter()
                            .map(|remap| ValueRemap {
                                old: value(&remap.old),
                                new: value(&remap.new),
                            })
                            .collect(),
                    )
                })
                .collect();
            (block, properties)
        })
        .collect()
}

pub fn convert(raw: RawSchema) -> Schema {
    Schema {
        version: Version {
            major: raw.max_version_major,
            minor: raw.max_version_minor,
            patch: raw.max_version_patch,
            revision: raw.max_version_revision,
        },
        renamed_ids: raw.renamed_ids,
        added_properties: raw.added_properties.iter().map(|(block, properties)| (block.clone(), values(properties))).collect(),
        removed_properties: raw.removed_properties,
        renamed_properties: raw.renamed_properties,
        remapped_property_values: inline_value_remaps(raw.remapped_property_values, &raw.remapped_property_values_index),
        flattened_properties: raw.flattened_properties.into_iter().map(|(block, rule)| (block, flatten_rule(rule))).collect(),
        remapped_states: raw.remapped_states.into_iter().map(|(block, rules)| (block, rules.into_iter().map(state_remap).collect())).collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value as Json, json};

    fn transform(raw: Json) -> Json {
        let raw: RawSchema = serde_json::from_value(raw).expect("the schema parses");
        serde_json::to_value(convert(raw)).expect("the schema serializes")
    }

    #[test]
    fn keys_become_snake_case_and_empty_values_disappear() {
        let schema = transform(json!({
            "maxVersionMajor": 1, "maxVersionMinor": 21, "maxVersionPatch": 0, "maxVersionRevision": 25,
            "renamedIds": {"minecraft:a": "minecraft:b"},
            "removedProperties": {},
            "addedProperties": {"minecraft:bell": {"attachment": {"string": "standing"}, "deprecated": {"int": 0}, "lit": {"byte": 1}}},
        }));
        assert_eq!(
            schema,
            json!({
                "version": "1.21.0.25",
                "renamed_ids": {"minecraft:a": "minecraft:b"},
                "added_properties": {"minecraft:bell": {"attachment": "standing", "deprecated": 0, "lit": true}},
            })
        );
    }

    #[test]
    fn value_remaps_are_inlined_as_pairs() {
        let schema = transform(json!({
            "maxVersionMajor": 1, "maxVersionMinor": 12, "maxVersionPatch": 0, "maxVersionRevision": 1,
            "remappedPropertyValues": {"minecraft:dirt": {"dirt_type": "dirt_type_00"}},
            "remappedPropertyValuesIndex": {"dirt_type_00": [{"old": {"int": 0}, "new": {"string": "normal"}}, {"old": {"int": 1}, "new": {"string": "coarse"}}]},
        }));
        assert_eq!(schema["remapped_property_values"], json!({"minecraft:dirt": {"dirt_type": [[0, "normal"], [1, "coarse"]]}}));
    }

    #[test]
    fn flatten_rules_and_state_remaps_drop_what_is_default() {
        let schema = transform(json!({
            "maxVersionMajor": 1, "maxVersionMinor": 19, "maxVersionPatch": 60, "maxVersionRevision": 0,
            "flattenedProperties": {"minecraft:wool": {"flattenedProperty": "color", "flattenedPropertyType": "string", "prefix": "minecraft:", "suffix": "_wool", "flattenedValueRemaps": {"silver": "light_gray"}}},
            "remappedStates": {"minecraft:end_rod": [{"oldState": {"facing_direction": {"int": 6}}, "newName": "minecraft:light_block", "newState": {"block_light_level": {"int": 14}}}, {"copiedState": ["facing_direction"], "newName": "minecraft:end_rod", "newState": null, "oldState": null}]},
        }));
        assert_eq!(
            schema["flattened_properties"]["minecraft:wool"],
            json!({"flattened_property": "color", "prefix": "minecraft:", "suffix": "_wool", "flattened_value_remaps": {"silver": "light_gray"}})
        );
        assert_eq!(
            schema["remapped_states"]["minecraft:end_rod"][1],
            json!({"new_name": "minecraft:end_rod", "copied_state": ["facing_direction"]})
        );
    }

    #[test]
    fn an_unknown_upstream_field_is_an_error() {
        let raw = json!({"maxVersionMajor": 1, "maxVersionMinor": 1, "maxVersionPatch": 0, "maxVersionRevision": 0, "somethingNew": {}});
        assert!(serde_json::from_value::<RawSchema>(raw).is_err());
    }
}
