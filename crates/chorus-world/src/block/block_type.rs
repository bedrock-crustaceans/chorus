// use crate::block::block_attributes::BlockAttributes;
// use crate::block::block_permutation::BlockPermutation;
// use crate::block::component::block_components::BlockComponents;
// use crate::block::state::block_state_type::BlockStateType;
// use crate::block::state::block_state_value::BlockStateValue;
// use crate::error::block_permutation_create::BlockPermutationCreateError;
// use chorus_util::utils::hash_utils::HashUtils;
// use std::collections::HashMap;
//
// #[derive(Clone, Debug, PartialEq)]
// pub struct BlockType {
//     identifier: String,
//     states: Vec<BlockStateType>,
//
//     components: BlockComponents,
//     attributes: BlockAttributes,
//
//     special_value_map: HashMap<i16, BlockPermutation>,
//     special_value_bits: u8,
//
//     default_permutation: BlockPermutation,
// }
//
// impl BlockType {
//     pub fn create(
//         identifier: &str,
//         states: Vec<BlockStateType>,
//         components: BlockComponents,
//         attributes: BlockAttributes,
//     ) -> Result<Self, BlockPermutationCreateError> {
//         let identifier = identifier.to_string();
//
//         let mut special_value_bits: u8 = 0;
//         for val in &states {
//             special_value_bits += val.get_bit_size();
//         }
//
//         if special_value_bits > 16 {
//             return Err(BlockPermutationCreateError {
//                 identifier: String::from(identifier),
//                 states: states.clone(),
//             });
//         }
//
//         if let Some((state_map, default_state)) =
//             Self::init_states(identifier.clone(), states.clone())
//         {
//             Ok(Self {
//                 identifier,
//                 states,
//
//                 components,
//                 attributes,
//
//                 special_value_map: state_map
//                     .iter()
//                     .map(|(_, v)| (v.get_special_value(), v.clone()))
//                     .collect::<HashMap<i16, BlockPermutation>>(),
//                 special_value_bits,
//
//                 default_permutation: default_state,
//             })
//         } else {
//             Err(BlockPermutationCreateError {
//                 identifier: String::from(identifier),
//                 states: states.clone(),
//             })
//         }
//     }
//
//     fn init_states(
//         identifier: String,
//         properties: Vec<BlockStateType>,
//     ) -> Option<(HashMap<i32, BlockPermutation>, BlockPermutation)> {
//         if properties.is_empty() {
//             let block_permutation =
//                 BlockPermutation::new(identifier.clone(), vec![], None, None, None);
//             let mut special_value_map = HashMap::new();
//             special_value_map.insert(block_permutation.get_hash(), block_permutation.clone());
//             return Some((special_value_map, block_permutation));
//         }
//
//         let size = properties.len();
//
//         let mut block_permutations: HashMap<i32, BlockPermutation> = HashMap::new();
//         let mut indices: Vec<usize> = vec![0; size];
//
//         loop {
//             let mut states: Vec<BlockStateValue> = vec![];
//             for i in 0..size {
//                 let r#type = &properties[i];
//                 let val = match r#type {
//                     BlockStateType::Boolean { valid_values, .. } => {
//                         BlockStateValue::create_boolean(
//                             r#type.clone(),
//                             valid_values[indices[i]].clone(),
//                         )
//                         .unwrap()
//                     }
//                     BlockStateType::Int { valid_values, .. } => BlockStateValue::create_int(
//                         r#type.clone(),
//                         valid_values[indices[i]].clone(),
//                     )
//                     .unwrap(),
//                     BlockStateType::Enum { valid_values, .. } => BlockStateValue::create_enum(
//                         r#type.clone(),
//                         valid_values[indices[i]].clone(),
//                     )
//                     .unwrap(),
//                 };
//                 states.push(val)
//             }
//             let state = BlockPermutation::new(identifier.clone(), states, None, None, None);
//
//             block_permutations.insert(state.get_hash(), state);
//
//             let mut next = size as isize - 1;
//             while next >= 0
//                 && (indices[next as usize] + 1
//                     >= match (&properties[next as usize]) {
//                         BlockStateType::Boolean { valid_values, .. } => valid_values.len(),
//                         BlockStateType::Int { valid_values, .. } => valid_values.len(),
//                         BlockStateType::Enum { valid_values, .. } => valid_values.len(),
//                     })
//             {
//                 next -= 1;
//             }
//
//             if next < 0 {
//                 break;
//             }
//
//             indices[next as usize] += 1;
//
//             for i in next as usize + 1..size {
//                 indices[i] = 0
//             }
//         }
//
//         let default_permutation_hash = HashUtils::compute_block_permutation_hash(
//             identifier.clone(),
//             properties
//                 .iter()
//                 .map(|v| v.create_default())
//                 .collect::<Vec<_>>(),
//         );
//
//         if let Some(state) = block_permutations.get(&default_permutation_hash) {
//             Some((block_permutations.clone(), state.clone()))
//         } else {
//             None
//         }
//     }
//
//     pub fn get_identifier(&self) -> String {
//         self.identifier.clone()
//     }
//
//     pub fn get_properties(&self) -> Vec<BlockStateType> {
//         self.states.clone()
//     }
//
//     pub fn get_special_value_map(&self) -> HashMap<i16, BlockPermutation> {
//         self.special_value_map.clone()
//     }
//
//     pub fn get_special_value_bits(&self) -> u8 {
//         self.special_value_bits.clone()
//     }
//
//     pub fn get_default_permutation(&self) -> &BlockPermutation {
//         &self.default_permutation
//     }
//
//     pub fn get_block_permutation(&self, special_value: i16) -> Option<BlockPermutation> {
//         self.special_value_map.get(&special_value).cloned()
//     }
//
//     pub fn get_block_permutation_with_state(
//         &self,
//         state: BlockStateValue,
//     ) -> Option<BlockPermutation> {
//         self.default_permutation
//             .set_property_value(self.clone(), state)
//     }
//
//     pub fn get_block_permutation_with_states(
//         &self,
//         states: Vec<BlockStateValue>,
//     ) -> Option<BlockPermutation> {
//         self.default_permutation
//             .set_property_values(self.clone(), states)
//     }
//
//     pub fn has_block_permutation(&self, permutation: &BlockPermutation) -> bool {
//         self.special_value_map
//             .contains_key(&permutation.get_special_value())
//     }
//
//     pub fn has_block_permutation_special_value(&self, special_value: i16) -> bool {
//         self.special_value_map.contains_key(&special_value)
//     }
//
//     pub fn has_state(&self, state: BlockStateType) -> bool {
//         self.states.contains(&state)
//     }
// }
