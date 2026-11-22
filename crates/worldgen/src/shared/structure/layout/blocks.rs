use std::collections::BTreeMap;

use chorus_block::block_upgrade::upgrade_block_state;
use chorus_structure_data::{StatePool, StatePoolBuilder};
use serde_json::{Map, Value};

use super::documents::PoolDocuments;
use super::fault::{Fault, PoolError};
use super::rule_documents::parse_processor_list;
use crate::shared::structure::data::jigsaw::State;

const NAMESPACE: &str = "minecraft:";
const LEGACY_VERSION: i32 = 0;
const BDS_UNSTATED_STATES: [(&str, &str); 2] = [("suspicious_gravel", "brushed_progress=0,hanging=1"), ("suspicious_sand", "brushed_progress=0,hanging=1")];

type States = BTreeMap<String, String>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Output {
    pub name: String,
    pub states: String,
}

pub fn output_state(name: &str, states: Option<&Map<String, Value>>) -> Option<Output> {
    let name = bare(name);
    let Some(states) = states else {
        return Some(unstated(name));
    };
    let wanted: Option<States> = states.iter().map(|(key, value)| Some((key.clone(), render(value)?))).collect();
    let exact = Output {
        name: name.to_owned(),
        states: join(&wanted?),
    };
    Some(upgraded(name, states).filter(|upgraded| *upgraded != exact).unwrap_or(exact))
}

pub fn output_state_of(name: &str, states: &BTreeMap<String, State>) -> Option<Output> {
    if states.is_empty() {
        return output_state(name, None);
    }
    let map: Map<String, Value> = states
        .iter()
        .map(|(key, value)| {
            let value = match value {
                State::Bool(flag) => Value::Bool(*flag),
                State::Int(number) => Value::from(*number),
                State::Text(text) => Value::String(text.clone()),
            };
            (key.clone(), value)
        })
        .collect();
    output_state(name, Some(&map))
}

fn unstated(name: &str) -> Output {
    let states = BDS_UNSTATED_STATES.iter().find(|(known, _)| *known == name).map_or("", |(_, states)| states);
    Output {
        name: name.to_owned(),
        states: states.to_owned(),
    }
}

pub fn declare_outputs(documents: &PoolDocuments, pool: &mut StatePoolBuilder) -> Result<(), PoolError> {
    let mut found = Vec::new();
    let mut names: Vec<&String> = documents.processors.keys().collect();
    names.sort();
    for name in names {
        let steps = parse_processor_list(&documents.processors[name]).map_err(|fault| fault.in_document(name.as_str()))?;
        for step in &steps {
            found.extend(step.rules().iter().map(|rule| rule.output.clone()));
        }
    }
    for family in &documents.families {
        for set in family.rule_sets.values() {
            found.extend(set.rules.iter().map(|rule| output_state_of(&rule.to, &rule.to_states)));
        }
    }
    for output in found.into_iter().flatten() {
        pool.intern(&output.name, &output.states, false);
    }
    Ok(())
}

pub struct BlockResolver<'a> {
    pool: &'a StatePool,
}

impl<'a> BlockResolver<'a> {
    pub fn new(pool: &'a StatePool) -> Self {
        Self { pool }
    }

    pub fn resolve(&self, output: Option<Output>) -> Result<Option<u16>, Fault> {
        let Some(output) = output else { return Ok(None) };
        self.pool
            .find(&output.name, &output.states, false)
            .map(Some)
            .ok_or_else(|| Fault::undeclared("output_state", format!("{}[{}]", output.name, output.states)))
    }

    pub fn resolve_states(&self, name: &str, states: &BTreeMap<String, State>) -> Result<Option<u16>, Fault> {
        self.resolve(output_state_of(name, states))
    }
}

fn bare(name: &str) -> &str {
    name.strip_prefix(NAMESPACE).unwrap_or(name)
}

fn join(states: &States) -> String {
    states.iter().map(|(key, value)| format!("{key}={value}")).collect::<Vec<_>>().join(",")
}

fn upgraded(name: &str, states: &Map<String, Value>) -> Option<Output> {
    let mut compound = nbtx::Compound::new();
    for (key, value) in states {
        compound.insert(key.as_str().into(), nbt_value(value)?);
    }
    let upgraded = upgrade_block_state(&format!("{NAMESPACE}{name}"), compound, LEGACY_VERSION);
    let wanted: Option<States> = upgraded.states.iter().map(|(key, value)| Some((key.to_string(), render_nbt(value)?))).collect();
    Some(Output {
        name: bare(&upgraded.name).to_owned(),
        states: join(&wanted?),
    })
}

fn render(value: &Value) -> Option<String> {
    match value {
        Value::Bool(flag) => Some(u8::from(*flag).to_string()),
        Value::Number(number) => Some(number.to_string()),
        Value::String(text) => Some(text.clone()),
        _ => None,
    }
}

fn render_nbt(value: &nbtx::Value) -> Option<String> {
    match value {
        nbtx::Value::Byte(number) => Some(number.to_string()),
        nbtx::Value::Short(number) => Some(number.to_string()),
        nbtx::Value::Int(number) => Some(number.to_string()),
        nbtx::Value::String(text) => Some(text.to_string()),
        _ => None,
    }
}

fn nbt_value(value: &Value) -> Option<nbtx::Value> {
    match value {
        Value::Bool(flag) => Some(nbtx::Value::Byte(i8::from(*flag))),
        Value::Number(number) => number.as_i64().map(|integer| nbtx::Value::Int(integer as i32)),
        Value::String(text) => Some(nbtx::Value::String(text.as_str().into())),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn states(value: serde_json::Value) -> Map<String, Value> {
        value.as_object().expect("a state map").clone()
    }

    #[test]
    fn an_unstated_archaeology_block_keeps_its_hanging_state() {
        let output = output_state("minecraft:suspicious_gravel", None).unwrap();
        assert_eq!((output.name.as_str(), output.states.as_str()), ("suspicious_gravel", "brushed_progress=0,hanging=1"));
        let sand = output_state("minecraft:suspicious_sand", None).unwrap();
        assert_eq!(sand.states, "brushed_progress=0,hanging=1");
        let plain = output_state("minecraft:dirt", None).unwrap();
        assert_eq!((plain.name.as_str(), plain.states.as_str()), ("dirt", ""));
    }

    #[test]
    fn a_legacy_output_is_upgraded_to_its_current_block() {
        let output = output_state("minecraft:dirt", Some(&states(serde_json::json!({"dirt_type": "coarse"})))).unwrap();
        assert_eq!((output.name.as_str(), output.states.as_str()), ("coarse_dirt", ""));
    }
}
