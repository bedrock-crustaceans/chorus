use super::super::site::Log;
use crate::shared::blocks::{BlockId, BlockTable, State};
use atomicow::CowArc;
use chorus_block::state::block_state::BlockState;
use serde_json::{Map, Value};

pub const MALFORMED: &str = "malformed tree feature";

const VALUE_KEY: &str = "value";

const WEIGHT_KEY: &str = "weight";

const NAME_KEY: &str = "name";

const STATES_KEY: &str = "states";

pub fn object(value: &'static Value) -> Result<&'static Map<String, Value>, &'static str> {
    value.as_object().ok_or(MALFORMED)
}

pub(super) fn number(value: &Value, key: &str, default: i32) -> i32 {
    value.get(key).and_then(Value::as_f64).map_or(default, |found| found as i32)
}

pub(super) fn required(value: &Value, key: &str) -> Result<i32, &'static str> {
    value.get(key).and_then(Value::as_f64).map(|found| found as i32).ok_or(MALFORMED)
}

pub(super) fn float(value: &Value, key: &str, default: f64) -> f32 {
    value.get(key).and_then(Value::as_f64).unwrap_or(default) as f32
}

fn state_value(value: &'static Value) -> Result<BlockState, &'static str> {
    match value {
        Value::Bool(flag) => Ok(BlockState::Bool(*flag)),
        Value::Number(number) => Ok(BlockState::Int(number.as_i64().ok_or(MALFORMED)? as i32)),
        Value::String(text) => Ok(crate::shared::blocks::text(text)),
        _ => Err(MALFORMED),
    }
}

pub fn block(table: &mut BlockTable, value: &'static Value) -> Result<BlockId, &'static str> {
    if let Some(name) = value.as_str() {
        return Ok(table.name(name));
    }
    let name = value.get(NAME_KEY).and_then(Value::as_str).ok_or(MALFORMED)?;
    let states: Vec<State> = match value.get(STATES_KEY) {
        Some(states) => object(states)?
            .iter()
            .map(|(key, value)| Ok((CowArc::Static(key.as_str()), state_value(value)?)))
            .collect::<Result<_, &'static str>>()?,
        None => Vec::new(),
    };
    Ok(table.get(name, &states))
}

pub(super) fn log(table: &mut BlockTable, value: &'static Value) -> Result<Log, &'static str> {
    let name = value.as_str().or_else(|| value.get(NAME_KEY).and_then(Value::as_str)).ok_or(MALFORMED)?;
    Ok(Log::new(table, name))
}

pub(super) fn intervals(height: &Value) -> Vec<i32> {
    height
        .get("intervals")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_i64)
        .map(|interval| interval as i32)
        .collect()
}

pub(super) fn weighted_values(value: Option<&Value>, default: i32) -> Vec<(i32, i32)> {
    value
        .and_then(Value::as_array)
        .map(|entries| {
            entries
                .iter()
                .map(|entry| {
                    (
                        entry.get(VALUE_KEY).and_then(Value::as_i64).unwrap_or(default as i64) as i32,
                        entry.get(WEIGHT_KEY).and_then(Value::as_i64).unwrap_or(1) as i32,
                    )
                })
                .collect()
        })
        .unwrap_or_else(|| vec![(default, 1)])
}

pub(super) fn weighted_blocks(table: &mut BlockTable, value: &'static Value) -> Result<Vec<(BlockId, i32)>, &'static str> {
    value
        .as_array()
        .ok_or(MALFORMED)?
        .iter()
        .map(|pair| Ok((block(table, &pair[0])?, pair[1].as_i64().ok_or(MALFORMED)? as i32)))
        .collect()
}
