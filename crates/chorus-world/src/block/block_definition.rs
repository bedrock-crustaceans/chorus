use crate::block::block_permutation::BlockPermutation;
use crate::block::component::block_component::BlockComponent;
use crate::block::component::block_components::BlockComponents;
use crate::block::state::block_state::{BlockState, BlockStateDefinition};
use atomicow::CowArc;
use chorus_util::utils::identifier::Identifier;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

#[derive(Debug)]
pub struct BlockDefinition {
    pub identifier: CowArc<'static, str>,
    pub states: CowArc<'static, [CowArc<'static, BlockStateDefinition>]>,
    pub components: CowArc<'static, [CowArc<'static, dyn BlockComponent>]>,
    pub permutations: CowArc<'static, [CowArc<'static, BlockPermutationDefinition>]>,
}

impl BlockDefinition {
    pub fn new(
        identifier: impl AsRef<str>,
        states: impl IntoIterator<Item = BlockStateDefinition>,
        components: impl IntoIterator<Item = Arc<dyn BlockComponent>>,
        permutations: impl IntoIterator<Item = BlockPermutationDefinition>,
    ) -> Self {
        Self {
            identifier: CowArc::Owned(Arc::from(identifier.as_ref())),
            states: CowArc::Owned(states.into_iter().map(|v| CowArc::Owned(Arc::new(v))).collect::<Vec<_>>().into()),
            components: CowArc::Owned(components.into_iter().map(CowArc::Owned).collect::<Vec<_>>().into()),
            permutations: CowArc::Owned(permutations.into_iter().map(|p| CowArc::Owned(Arc::new(p))).collect::<Vec<_>>().into()),
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        Identifier::validate(&self.identifier)?;

        let size: usize = self.states.iter().fold(1, |acc, s| acc * s.values_len());
        if size > u16::MAX as usize {
            return Err(format!("found {size} permutations, which exceeds the limit of 65535 permutations",));
        }

        let mut seen = HashSet::new();
        for state in self.states.as_ref() {
            let ident = state.identifier().as_ref();
            if !seen.insert(ident) {
                return Err(format!("found duplicate state identifier {:?}", ident));
            }
            state.validate()?;
        }
        Ok(())
    }

    pub fn generate(&self) -> (i32, HashMap<i32, BlockPermutation>, HashMap<i32, BlockComponents>) {
        let state_combinations = self.states.iter().fold(vec![HashMap::new()], |acc, state| {
            acc.into_iter()
                .flat_map(|map| {
                    state.get_values().into_iter().map(move |val| {
                        let mut m = map.clone();
                        m.insert(state.identifier().clone(), val);
                        m
                    })
                })
                .collect()
        });

        let permutations: Vec<BlockPermutation> = state_combinations.into_iter().map(move |states| BlockPermutation::new(self, states)).collect();

        let default = permutations.iter().find(|p| p.get_index() == 0).unwrap().get_hash();

        let components: HashMap<i32, BlockComponents> = permutations
            .iter()
            .map(|permutation| {
                let hash = permutation.get_hash();
                let mut components = BlockComponents::new();

                for c in self.components.as_ref() {
                    components.insert(c.clone());
                }

                for p in self.permutations.as_ref() {
                    if (p.condition)(permutation.get_states()) {
                        for c in p.components.as_ref() {
                            components.insert(c.clone());
                        }
                    }
                }

                (hash, components)
            })
            .collect();

        let permutations: HashMap<i32, BlockPermutation> = permutations.into_iter().map(|p| (p.get_hash(), p)).collect();

        (default, permutations, components)
    }
}

impl From<BlockDefinition> for CowArc<'_, BlockDefinition> {
    fn from(def: BlockDefinition) -> Self {
        CowArc::Owned(def.into())
    }
}

#[derive(Debug)]
pub struct BlockPermutationDefinition {
    pub condition: fn(&HashMap<CowArc<'static, str>, BlockState>) -> bool,
    pub components: CowArc<'static, [CowArc<'static, dyn BlockComponent>]>,
}

impl BlockPermutationDefinition {
    pub fn new(condition: fn(&HashMap<CowArc<'static, str>, BlockState>) -> bool, components: impl IntoIterator<Item = Arc<dyn BlockComponent>>) -> Self {
        Self {
            condition,
            components: CowArc::Owned(components.into_iter().map(CowArc::Owned).collect::<Vec<_>>().into()),
        }
    }
}
