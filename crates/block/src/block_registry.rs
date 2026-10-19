use crate::block_definition::BlockDefinition;
use crate::block_permutation::BlockPermutation;
use crate::component::block_components::BlockComponents;
use crate::r#impl::DEFINITIONS;
use crate::state::block_state::BlockState;
use atomicow::CowArc;
use bevy_ecs::prelude::{Commands, Resource};
use rand::prelude::IteratorRandom;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{info, warn};

#[derive(Resource)]
pub struct BlockRegistry {
    definitions: HashMap<CowArc<'static, str>, CowArc<'static, BlockDefinition>>,

    default_hash: HashMap<CowArc<'static, str>, i32>,
    indexed_hash: HashMap<(CowArc<'static, str>, u16), i32>,

    permutations: Arc<HashMap<i32, BlockPermutation>>,
    components: HashMap<i32, BlockComponents>,
}

impl Default for BlockRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl BlockRegistry {
    pub fn new() -> Self {
        Self {
            definitions: HashMap::new(),

            default_hash: HashMap::new(),
            indexed_hash: HashMap::new(),

            permutations: Arc::new(HashMap::new()),
            components: HashMap::new(),
        }
    }

    #[tracing::instrument(name = "registry", skip_all)]
    pub fn init(mut commands: Commands) {
        let mut registry = Self::new();

        registry.register_all(DEFINITIONS.iter().copied());

        commands.insert_resource(registry);
    }

    pub fn register<D>(&mut self, definition: D)
    where
        D: Into<CowArc<'static, BlockDefinition>>,
    {
        let definition = definition.into();

        if let Err(message) = definition.validate() {
            warn!("failed to register {:?}: {}", definition.identifier, message);
            return;
        }

        let (default, permutations, components) = definition.generate();

        self.default_hash.insert(definition.identifier.clone(), default);

        let indexed_hashes: HashMap<(CowArc<'static, str>, u16), i32> = permutations.iter().map(|(hash, p)| ((definition.identifier.clone(), p.get_index()), *hash)).collect();

        self.indexed_hash.extend(indexed_hashes);
        Arc::make_mut(&mut self.permutations).extend(permutations);
        self.components.extend(components);

        self.definitions.insert(definition.identifier.clone(), definition);
    }

    pub fn register_all<I, D>(&mut self, definitions: I)
    where
        I: IntoIterator<Item = D>,
        D: Into<CowArc<'static, BlockDefinition>>,
    {
        let before = self.definitions.len();

        for def in definitions {
            self.register(def);
        }

        info!("registered {} blocks", self.definitions.len() - before);
    }

    pub fn get_block_id(&self, identifier: &str) -> Option<i32> {
        self.default_hash.get(identifier).copied()
    }

    pub fn get_block_id_with_states(&self, identifier: &str, states: &[(&str, BlockState)]) -> Option<i32> {
        let definition = self.definitions.get(identifier)?;
        let mut values = self.permutations.get(self.default_hash.get(identifier)?)?.get_states().clone();

        for (name, value) in states {
            let state = definition.states.iter().find(|state| state.identifier().as_ref() == *name)?;
            if !(0..state.values_len() as i32).contains(&state.index_of(value)) {
                return None;
            }
            values.insert(state.identifier().clone(), value.clone());
        }

        let index = BlockPermutation::compute_index(&values, definition.states.as_ref());
        self.indexed_hash.get(&(definition.identifier.clone(), index)).copied()
    }

    pub fn get_components(&self, hash: i32) -> Option<&BlockComponents> {
        self.components.get(&hash)
    }

    pub fn get_permutation(&self, hash: i32) -> Option<&BlockPermutation> {
        self.permutations.get(&hash)
    }

    pub fn permutation_table(&self) -> Arc<HashMap<i32, BlockPermutation>> {
        Arc::clone(&self.permutations)
    }

    pub fn permutations(&self) -> impl Iterator<Item = &BlockPermutation> {
        self.permutations.values()
    }

    pub fn get_random(&self) -> i32 {
        *self.default_hash.values().choose(&mut rand::rng()).expect("no hash found")
    }

    pub fn get_all_block_ids(&self) -> Vec<i32> {
        self.default_hash.values().copied().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::block_id;

    fn registry() -> BlockRegistry {
        let mut registry = BlockRegistry::new();
        registry.register_all(DEFINITIONS.iter().copied());
        registry
    }

    #[test]
    fn every_bundled_definition_registers() {
        let registry = registry();
        assert_eq!(registry.definitions.len(), DEFINITIONS.len());
        assert_eq!(registry.get_all_block_ids().len(), DEFINITIONS.len());
    }

    #[test]
    fn known_blocks_resolve_by_identifier() {
        let registry = registry();
        for identifier in [block_id::AIR, block_id::STONE, block_id::GRASS_BLOCK, block_id::DIRT] {
            assert!(registry.get_block_id(identifier).is_some(), "{identifier}");
        }
        assert_eq!(registry.get_block_id("minecraft:not_a_block"), None);
    }

    #[test]
    fn known_blocks_have_distinct_ids() {
        let registry = registry();
        let ids: Vec<i32> = [block_id::AIR, block_id::STONE, block_id::GRASS_BLOCK].iter().map(|id| registry.get_block_id(id).unwrap()).collect();
        assert_ne!(ids[0], ids[1]);
        assert_ne!(ids[1], ids[2]);
        assert_ne!(ids[0], ids[2]);
        assert_eq!(ids[0], -604749536);
    }

    #[test]
    fn ids_map_back_to_their_permutation_and_components() {
        let registry = registry();
        for identifier in [block_id::AIR, block_id::STONE, block_id::GRASS_BLOCK] {
            let id = registry.get_block_id(identifier).unwrap();
            assert_eq!(registry.get_permutation(id).unwrap().get_identifier(), identifier);
            assert_eq!(registry.get_permutation(id).unwrap().get_hash(), id);
            assert!(registry.get_components(id).is_some());
        }
        assert!(registry.get_permutation(12345).is_none());
        assert!(registry.get_components(12345).is_none());
    }

    #[test]
    fn registering_twice_produces_the_same_ids() {
        let a = registry();
        let b = registry();
        for identifier in [block_id::AIR, block_id::STONE, block_id::GRASS_BLOCK, block_id::ACACIA_LOG] {
            assert_eq!(a.get_block_id(identifier), b.get_block_id(identifier));
        }
    }

    #[test]
    fn state_values_select_the_permutation() {
        let registry = registry();
        let default = registry.get_block_id(block_id::ACACIA_LOG).unwrap();
        let y = registry.get_block_id_with_states(block_id::ACACIA_LOG, &[("pillar_axis", "y".into())]).unwrap();
        let x = registry.get_block_id_with_states(block_id::ACACIA_LOG, &[("pillar_axis", "x".into())]).unwrap();
        let z = registry.get_block_id_with_states(block_id::ACACIA_LOG, &[("pillar_axis", "z".into())]).unwrap();

        assert_eq!(default, y);
        assert_eq!(registry.get_permutation(x).unwrap().get_state_value("pillar_axis"), Some(&BlockState::from("x")));
        assert_ne!(x, z);
        assert_ne!(x, y);
    }

    #[test]
    fn unknown_states_and_values_are_rejected() {
        let registry = registry();
        assert_eq!(registry.get_block_id_with_states(block_id::ACACIA_LOG, &[("nope", "y".into())]), None);
        assert_eq!(registry.get_block_id_with_states(block_id::ACACIA_LOG, &[("pillar_axis", "w".into())]), None);
        assert_eq!(registry.get_block_id_with_states("minecraft:not_a_block", &[]), None);
    }

    #[test]
    fn no_states_means_the_default_permutation() {
        let registry = registry();
        assert_eq!(registry.get_block_id_with_states(block_id::STONE, &[]), registry.get_block_id(block_id::STONE));
    }

    #[test]
    fn an_invalid_definition_is_skipped() {
        let mut registry = BlockRegistry::new();
        registry.register(BlockDefinition::new("not an identifier", [], [], []));
        assert!(registry.get_all_block_ids().is_empty());
    }

    #[test]
    fn get_random_returns_a_registered_id() {
        let registry = registry();
        assert!(registry.get_permutation(registry.get_random()).is_some());
    }
}
