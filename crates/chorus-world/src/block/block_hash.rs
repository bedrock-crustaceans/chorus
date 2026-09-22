use crate::block::state::block_state::BlockState;
use atomicow::CowArc;
use chorus_util::utils::hash_utils::HashUtils::hash_nbt;
use std::collections::HashMap;

pub fn hash_block_permutation(identifier: &str, states: &HashMap<CowArc<'static, str>, BlockState>) -> i32 {
    if identifier == "minecraft:unknown" {
        return -2;
    }

    let mut states_tag: HashMap<String, nbtx::Value> = HashMap::new();
    for (id, val) in states {
        match val {
            BlockState::Bool(val) => {
                states_tag.insert(id.to_string(), nbtx::Value::Byte(if *val { 1 } else { 0 }));
            }
            BlockState::Int(val) => {
                states_tag.insert(id.to_string(), nbtx::Value::Int(*val));
            }
            BlockState::Enum(val) => {
                states_tag.insert(id.to_string(), nbtx::Value::String(val.to_string()));
            }
        }
    }

    let mut tag: HashMap<String, nbtx::Value> = HashMap::new();

    tag.insert(String::from("name"), nbtx::Value::String(identifier.to_string()));
    tag.insert(String::from("states"), nbtx::Value::Compound(states_tag));

    hash_nbt(&tag)
}
