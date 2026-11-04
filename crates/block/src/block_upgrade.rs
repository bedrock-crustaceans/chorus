use chorus_block_schema::{Bundle, BundledSchema};
pub use chorus_block_schema::{FlattenRule, FlattenedPropertyType, PropertyValue, StateRemap, ValueRemap};
use chorus_core::info::BLOCK_STATE_VERSION;
use std::collections::BTreeMap;
use std::sync::OnceLock;

const EMBEDDED_SCHEMAS: &str = include_str!("../resources/block_upgrade_schemas.json");

static UPGRADER: OnceLock<BlockStateUpgrader> = OnceLock::new();

pub fn upgrader() -> &'static BlockStateUpgrader {
    UPGRADER.get_or_init(|| BlockStateUpgrader::from_embedded().unwrap_or_else(|errors| panic!("embedded block upgrade schemas are invalid: {errors:?}")))
}

pub fn upgrade_block_state(name: &str, states: nbtx::Compound, version: i32) -> UpgradedBlockState {
    upgrader().upgrade(name, states, version)
}

#[derive(Clone, Debug, PartialEq)]
pub struct UpgradedBlockState {
    pub name: String,
    pub states: nbtx::Compound,
    pub version: i32,
}

#[derive(Clone, Debug)]
pub struct BlockStateUpgrader {
    by_version: Vec<(i32, Vec<BlockStateUpgradeSchema>)>,
    output_version: i32,
}

impl BlockStateUpgrader {
    pub fn from_embedded() -> Result<Self, Vec<String>> {
        let bundle: Bundle = serde_json::from_str(EMBEDDED_SCHEMAS).map_err(|error| vec![error.to_string()])?;
        let schemas = bundle.schemas.into_iter().map(BlockStateUpgradeSchema::from).filter(|schema| schema.version <= BLOCK_STATE_VERSION).collect();
        Ok(Self::from_schemas(schemas))
    }

    pub fn from_schemas(mut schemas: Vec<BlockStateUpgradeSchema>) -> Self {
        schemas.sort_by_key(|schema| (schema.version, schema.schema_id));

        let mut by_version: Vec<(i32, Vec<BlockStateUpgradeSchema>)> = Vec::new();
        for schema in schemas {
            match by_version.last_mut() {
                Some((version, list)) if *version == schema.version => list.push(schema),
                _ => by_version.push((schema.version, vec![schema])),
            }
        }

        let latest_schema_version = by_version.last().map_or(0, |(version, _)| *version);
        let output_version = latest_schema_version.max(BLOCK_STATE_VERSION);

        Self { by_version, output_version }
    }

    pub fn output_version(&self) -> i32 {
        self.output_version
    }

    pub fn schema_count(&self) -> usize {
        self.by_version.iter().map(|(_, schemas)| schemas.len()).sum()
    }

    pub fn upgrade(&self, name: &str, states: nbtx::Compound, version: i32) -> UpgradedBlockState {
        let input_version = version;
        let mut current = UpgradedBlockState {
            name: name.to_owned(),
            states,
            version,
        };

        for (result_version, schemas) in &self.by_version {
            if input_version > *result_version || (schemas.len() == 1 && input_version == *result_version) {
                continue;
            }
            for schema in schemas {
                current = Self::apply_schema(schema, current);
            }
        }

        if self.output_version > current.version {
            current.version = self.output_version;
        }

        current
    }

    fn apply_schema(schema: &BlockStateUpgradeSchema, current: UpgradedBlockState) -> UpgradedBlockState {
        if let Some((name, states)) = schema.apply_remapped_states(&current.name, &current.states) {
            return UpgradedBlockState {
                name,
                states,
                version: schema.version,
            };
        }

        let old_name = current.name.as_str();
        let mut states = current.states.clone();
        let mut new_name = None;

        if let Some(renamed) = schema.renamed_ids.get(old_name) {
            new_name = Some(renamed.clone());
        } else if let Some(rule) = schema.flattened_properties.get(old_name)
            && let Some((name, flattened_states)) = apply_flatten(rule, &states)
        {
            new_name = Some(name);
            states = flattened_states;
        }

        let mut state_changes = 0;
        states = schema.apply_added_properties(old_name, states, &mut state_changes);
        states = schema.apply_removed_properties(old_name, states, &mut state_changes);
        states = schema.apply_renamed_properties(old_name, states, &mut state_changes);
        states = schema.apply_remapped_property_values(old_name, states, &mut state_changes);

        if new_name.is_some() || state_changes > 0 {
            UpgradedBlockState {
                name: new_name.unwrap_or_else(|| old_name.to_owned()),
                states,
                version: schema.version,
            }
        } else {
            current
        }
    }
}

#[derive(Clone, Debug)]
pub struct BlockStateUpgradeSchema {
    pub schema_id: u32,
    pub version: i32,
    pub renamed_ids: BTreeMap<String, String>,
    pub added_properties: BTreeMap<String, BTreeMap<String, PropertyValue>>,
    pub removed_properties: BTreeMap<String, Vec<String>>,
    pub renamed_properties: BTreeMap<String, BTreeMap<String, String>>,
    pub remapped_property_values: BTreeMap<String, BTreeMap<String, Vec<ValueRemap>>>,
    pub flattened_properties: BTreeMap<String, FlattenRule>,
    pub remapped_states: BTreeMap<String, Vec<StateRemap>>,
}

impl From<BundledSchema> for BlockStateUpgradeSchema {
    fn from(bundled: BundledSchema) -> Self {
        let schema = bundled.schema;
        Self {
            schema_id: bundled.id,
            version: schema.version.packed(),
            renamed_ids: schema.renamed_ids,
            added_properties: schema.added_properties,
            removed_properties: schema.removed_properties,
            renamed_properties: schema.renamed_properties,
            remapped_property_values: schema.remapped_property_values,
            flattened_properties: schema.flattened_properties,
            remapped_states: schema.remapped_states,
        }
    }
}

impl BlockStateUpgradeSchema {
    fn apply_added_properties(&self, old_name: &str, mut states: nbtx::Compound, state_changes: &mut u32) -> nbtx::Compound {
        let Some(properties) = self.added_properties.get(old_name) else {
            return states;
        };
        for (name, value) in properties {
            if !states.contains_key(name.as_bytes()) {
                states.insert(name.as_str().into(), property_to_nbt(value));
                *state_changes += 1;
            }
        }
        states
    }

    fn apply_removed_properties(&self, old_name: &str, mut states: nbtx::Compound, state_changes: &mut u32) -> nbtx::Compound {
        let Some(properties) = self.removed_properties.get(old_name) else {
            return states;
        };
        for name in properties {
            if states.shift_remove(name.as_bytes()).is_some() {
                *state_changes += 1;
            }
        }
        states
    }

    fn apply_renamed_properties(&self, old_name: &str, mut states: nbtx::Compound, state_changes: &mut u32) -> nbtx::Compound {
        let Some(properties) = self.renamed_properties.get(old_name) else {
            return states;
        };

        for (old_property, new_property) in properties {
            let Some(old_value) = states.get(old_property.as_bytes()).cloned() else {
                continue;
            };
            states.shift_remove(old_property.as_bytes());
            let new_value = self.locate_new_property_value(old_name, old_property, &old_value);
            states.insert(new_property.as_str().into(), new_value);
            *state_changes += 1;
        }
        states
    }

    fn apply_remapped_property_values(&self, old_name: &str, mut states: nbtx::Compound, state_changes: &mut u32) -> nbtx::Compound {
        let Some(properties) = self.remapped_property_values.get(old_name) else {
            return states;
        };

        for old_property in properties.keys() {
            let Some(old_value) = states.get(old_property.as_bytes()).cloned() else {
                continue;
            };
            let new_value = self.locate_new_property_value(old_name, old_property, &old_value);
            if new_value != old_value {
                states.insert(old_property.as_str().into(), new_value);
                *state_changes += 1;
            }
        }
        states
    }

    fn locate_new_property_value(&self, old_name: &str, old_property: &str, old_value: &nbtx::Value) -> nbtx::Value {
        let Some(remaps) = self.remapped_property_values.get(old_name).and_then(|properties| properties.get(old_property)) else {
            return old_value.clone();
        };

        remaps.iter().find(|remap| property_matches(&remap.old, old_value)).map_or_else(|| old_value.clone(), |remap| property_to_nbt(&remap.new))
    }

    fn apply_remapped_states(&self, old_name: &str, old_states: &nbtx::Compound) -> Option<(String, nbtx::Compound)> {
        let remaps = self.remapped_states.get(old_name)?;

        for remap in remaps {
            let matches_filter = remap
                .old_state
                .as_ref()
                .is_none_or(|old_state| old_state.len() <= old_states.len() && old_state.iter().all(|(name, expected)| old_states.get(name.as_bytes()).is_some_and(|actual| property_matches(expected, actual))));
            if !matches_filter {
                continue;
            }

            let new_name = match (&remap.new_name, &remap.new_flattened_name) {
                (Some(name), _) => name.clone(),
                (None, Some(rule)) => apply_flatten_lossy(rule, old_name, old_states).0,
                (None, None) => continue,
            };

            let mut new_states = nbtx::Compound::new();
            for (name, value) in remap.new_state.iter().flatten() {
                new_states.insert(name.as_str().into(), property_to_nbt(value));
            }
            for name in &remap.copied_state {
                if let Some(value) = old_states.get(name.as_bytes()) {
                    new_states.insert(name.as_str().into(), value.clone());
                }
            }

            return Some((new_name, new_states));
        }

        None
    }
}

fn property_to_nbt(value: &PropertyValue) -> nbtx::Value {
    match value {
        PropertyValue::Byte(byte) => nbtx::Value::Byte(i8::from(*byte)),
        PropertyValue::Int(int) => nbtx::Value::Int(*int),
        PropertyValue::String(string) => nbtx::Value::String(string.as_str().into()),
    }
}

fn property_matches(value: &PropertyValue, actual: &nbtx::Value) -> bool {
    match (value, actual) {
        (PropertyValue::Byte(byte), nbtx::Value::Byte(actual)) => i8::from(*byte) == *actual,
        (PropertyValue::Int(int), nbtx::Value::Int(actual)) => int == actual,
        (PropertyValue::String(string), nbtx::Value::String(actual)) => actual.as_slice() == string.as_bytes(),
        _ => false,
    }
}

fn apply_flatten(rule: &FlattenRule, states: &nbtx::Compound) -> Option<(String, nbtx::Compound)> {
    let value = states.get(rule.flattened_property.as_bytes())?;
    let embed_key = match (&rule.flattened_property_type, value) {
        (FlattenedPropertyType::String, nbtx::Value::String(value)) => value.to_string(),
        (FlattenedPropertyType::Int, nbtx::Value::Int(value)) => value.to_string(),
        (FlattenedPropertyType::Byte, nbtx::Value::Byte(value)) => value.to_string(),
        _ => return None,
    };
    Some(apply_flatten_key(rule, &embed_key, states))
}

fn apply_flatten_lossy(rule: &FlattenRule, old_name: &str, states: &nbtx::Compound) -> (String, nbtx::Compound) {
    match apply_flatten(rule, states) {
        Some(result) => result,
        None => (old_name.to_owned(), states.clone()),
    }
}

fn apply_flatten_key(rule: &FlattenRule, embed_key: &str, states: &nbtx::Compound) -> (String, nbtx::Compound) {
    let embed = rule.flattened_value_remaps.get(embed_key).map_or(embed_key, String::as_str);
    let name = format!("{}{}{}", rule.prefix, embed, rule.suffix);
    let mut states = states.clone();
    states.shift_remove(rule.flattened_property.as_bytes());
    (name, states)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn version(major: i32, minor: i32, patch: i32, revision: i32) -> i32 {
        (major << 24) | (minor << 16) | (patch << 8) | revision
    }

    fn states(entries: &[(&str, nbtx::Value)]) -> nbtx::Compound {
        let mut compound = nbtx::Compound::new();
        for (name, value) in entries {
            compound.insert((*name).into(), value.clone());
        }
        compound
    }

    fn upgrade(name: &str, states: nbtx::Compound, version: i32) -> UpgradedBlockState {
        upgrader().upgrade(name, states, version)
    }

    #[test]
    fn schemas_newer_than_the_block_state_version_are_not_applied() {
        let bundle: Bundle = serde_json::from_str(EMBEDDED_SCHEMAS).expect("the bundle parses");
        let applicable = bundle.schemas.into_iter().map(BlockStateUpgradeSchema::from).filter(|schema| schema.version <= BLOCK_STATE_VERSION).count();
        assert_eq!(BlockStateUpgrader::from_embedded().expect("embedded schemas parse").schema_count(), applicable);
    }

    #[test]
    fn every_embedded_schema_parses() {
        let upgrader = BlockStateUpgrader::from_embedded().expect("embedded schemas parse");
        assert!(upgrader.schema_count() > 0);
        assert_eq!(upgrader.output_version(), BLOCK_STATE_VERSION);
    }

    #[test]
    fn renamed_block_ids_upgrade_to_their_current_identifier() {
        let upgraded = upgrade("minecraft:concretePowder", nbtx::Compound::new(), version(1, 18, 10, 1));
        assert_eq!(upgraded.name, "minecraft:concrete_powder");
        assert!(upgraded.version > version(1, 18, 10, 1));
    }

    #[test]
    fn flattened_blocks_become_the_modern_flat_identifier() {
        let upgraded = upgrade("minecraft:wool", states(&[("color", nbtx::Value::String("silver".into()))]), version(1, 19, 60, 0));
        assert_eq!(upgraded.name, "minecraft:light_gray_wool");
        assert!(upgraded.states.get("color".as_bytes()).is_none());
    }

    #[test]
    fn state_properties_are_renamed_and_removed() {
        let renamed = upgrade("minecraft:bone_block", states(&[("direction", nbtx::Value::String("x".into()))]), version(1, 12, 0, 1));
        assert_eq!(renamed.states.get("pillar_axis".as_bytes()), Some(&nbtx::Value::String("x".into())));
        assert!(renamed.states.get("direction".as_bytes()).is_none());

        let removed = upgrade("minecraft:skull", states(&[("no_drop_bit", nbtx::Value::Byte(1))]), version(1, 18, 0, 0));
        assert!(removed.states.get("no_drop_bit".as_bytes()).is_none());
    }

    #[test]
    fn property_values_are_remapped_through_the_value_index() {
        let upgraded = upgrade("minecraft:glow_lichen", states(&[("multi_face_direction_bits", nbtx::Value::Int(4))]), version(1, 18, 0, 0));
        assert_eq!(upgraded.states.get("multi_face_direction_bits".as_bytes()), Some(&nbtx::Value::Int(16)));
    }

    #[test]
    fn remapped_states_win_over_other_rules() {
        let upgraded = upgrade("minecraft:fence", states(&[("wood_type", nbtx::Value::String("spruce".into()))]), version(1, 19, 70, 0));
        assert_eq!(upgraded.name, "minecraft:spruce_fence");
        assert!(upgraded.states.get("wood_type".as_bytes()).is_none());
    }

    #[test]
    fn remapped_states_with_a_null_filter_match_every_state() {
        let upgraded = upgrade(
            "minecraft:end_rod",
            states(&[("facing_direction", nbtx::Value::Int(1)), ("ignored", nbtx::Value::Int(9))]),
            version(1, 12, 0, 1),
        );
        assert_eq!(upgraded.name, "minecraft:end_rod");
        assert_eq!(upgraded.states.get("facing_direction".as_bytes()), Some(&nbtx::Value::Int(1)));
        assert!(upgraded.states.get("ignored".as_bytes()).is_none());
    }

    #[test]
    fn up_to_date_blocks_are_left_alone() {
        let original = states(&[("facing_direction", nbtx::Value::Int(2))]);
        let upgraded = upgrade("minecraft:furnace", original.clone(), BLOCK_STATE_VERSION);
        assert_eq!(upgraded.name, "minecraft:furnace");
        assert_eq!(upgraded.states, original);
        assert_eq!(upgraded.version, BLOCK_STATE_VERSION);
    }
}
