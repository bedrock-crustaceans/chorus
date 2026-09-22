use crate::block::state::block_state::BlockState;
use std::collections::HashMap;
use std::fmt::Display;

#[derive(Debug)]
pub struct BlockPermutationCreateError {
    pub identifier: &'static str,
    pub states: HashMap<&'static str, BlockState>,
}

impl Display for BlockPermutationCreateError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "BlockPermutationCreateError {{ identifier: {:?}, states: {:?} }}", self.identifier, self.states)
    }
}

impl std::error::Error for BlockPermutationCreateError {}
