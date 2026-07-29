use crate::block::block_definition::BlockDefinition;
use crate::block::state::block_state::{BlockState, BlockStateDefinition};
use crate::info::BLOCK_STATE_VERSION;
use crate::utils::hash_utils::HashUtils;
use atomicow::CowArc;
use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct BlockPermutation {
    identifier: CowArc<'static, str>,
    states: HashMap<CowArc<'static, str>, BlockState>,
    index: u16,
    hash: i32,
}

impl From<BlockPermutation> for i32 {
    fn from(block_permutation: BlockPermutation) -> i32 {
        block_permutation.hash
    }
}

impl BlockPermutation {
    pub fn new(definition: &BlockDefinition, states: HashMap<CowArc<'static, str>, BlockState>) -> Self {
        let identifier = definition.identifier.clone();
        let index = Self::compute_index(&states, definition.states.as_ref());
        let hash = HashUtils::hash_block_permutation(identifier.as_ref(), &states);

        Self { identifier, states, index, hash }
    }
    
    pub fn get_identifier(&self) -> &str {
        &self.identifier
    }

    pub fn get_hash(&self) -> i32 {
        self.hash
    }

    pub fn get_index(&self) -> u16 {
        self.index
    }

    pub fn get_states(&self) -> &HashMap<CowArc<'static, str>, BlockState> {
        &self.states
    }

    pub fn get_state_value(&self, id: &str) -> Option<&BlockState> {
        self.states.get(id)
    }

    // pub fn set_property_value(
    //     &self,
    //     properties: BlockType,
    //     value: BlockStateValue,
    // ) -> Option<BlockPermutation> {
    //     let mut success = false;
    //     let mut new_property_values: Vec<BlockStateValue> = Vec::new();
    //     for v in &self.state_values {
    //         if (*v == value) {
    //             success = true;
    //             new_property_values.push(value.clone())
    //         } else {
    //             new_property_values.push(v.clone())
    //         }
    //     }
    //
    //     match success {
    //         true => self.get_new_block_state(properties, new_property_values),
    //         false => None,
    //     }
    // }

    // pub fn set_property_values(
    //     &self,
    //     properties: BlockType,
    //     values: Vec<BlockStateValue>,
    // ) -> Option<BlockPermutation> {
    //     let mut success_count: usize = 0;
    //
    //     let mut new_property_values: Vec<BlockStateValue> = Vec::new();
    //     'f: for v in &self.state_values {
    //         for j in &values {
    //             if (*v == *j) {
    //                 success_count += 1;
    //                 new_property_values.push(j.clone());
    //                 continue 'f;
    //             }
    //         }
    //         new_property_values.push(v.clone());
    //     }
    //
    //     match success_count == values.len() {
    //         true => self.get_new_block_state(properties, new_property_values),
    //         false => None,
    //     }
    // }

    pub fn compute_index(states: &HashMap<CowArc<'static, str>, BlockState>, defs: &[CowArc<'static, BlockStateDefinition>]) -> u16 {
        let mut sorted: Vec<_> = defs.iter().map(AsRef::as_ref).collect();
        sorted.sort_by_key(|d| d.identifier());

        let mut index: u16 = 0;
        for def in sorted {
            let value = &states[def.identifier()];

            let value_index = def.index_of(value) as u16;
            let radix = def.values_len() as u16;

            index = index * radix + value_index;
        }
        index
    }

    pub fn build_block_state_tag(identifier: &str, property_values: &HashMap<&str, BlockState>) -> HashMap<String, nbtx::Value> {
        let mut states: HashMap<String, nbtx::Value> = HashMap::new();
        for (id, val) in property_values {
            match val {
                BlockState::Bool(val) => {
                    states.insert(id.to_string(), nbtx::Value::Byte(if *val { 1 } else { 0 }));
                }
                BlockState::Int(val) => {
                    states.insert(id.to_string(), nbtx::Value::Int(*val));
                }
                BlockState::Enum(val) => {
                    states.insert(id.to_string(), nbtx::Value::String(val.to_string()));
                }
            }
        }

        let mut tag: HashMap<String, nbtx::Value> = HashMap::new();
        tag.insert(String::from("name"), nbtx::Value::String(identifier.to_owned()));
        tag.insert(String::from("states"), nbtx::Value::Compound(states));
        tag.insert(String::from("version"), nbtx::Value::Int(BLOCK_STATE_VERSION));
        tag
    }

    // fn get_new_block_state(
    //     &self,
    //     properties: BlockType,
    //     values: Vec<BlockStateValue>,
    // ) -> Option<BlockPermutation> {
    //     let bits: u8 = properties.get_special_value_bits();
    //     match (bits <= 16) {
    //         true => {
    //             properties.get_block_permutation(Self::compute_special_value(values, Some(bits)))
    //         }
    //         false => None,
    //     }
    // }
}
