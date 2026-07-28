use crate::block::block_definition::BlockDefinition;
use crate::block::block_permutation::BlockPermutation;
use crate::block::component::block_components::BlockComponents;
use crate::block::r#impl::DEFINITIONS;
use atomicow::CowArc;
use bevy_ecs::prelude::{Commands, Resource};
use rand::prelude::IteratorRandom;
use std::collections::HashMap;
use tracing::{info, warn};

#[derive(Resource)]
pub struct BlockRegistry {
    definitions: HashMap<CowArc<'static, str>, CowArc<'static, BlockDefinition>>,

    default_hash: HashMap<CowArc<'static, str>, i32>,
    indexed_hash: HashMap<(CowArc<'static, str>, u16), i32>,

    permutations: HashMap<i32, BlockPermutation>,
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

            permutations: HashMap::new(),
            components: HashMap::new(),
        }
    }

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
        self.permutations.extend(permutations);
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

    pub fn get_random(&self) -> i32 {
        *self.default_hash.values().choose(&mut rand::rng()).expect("no hash found")
    }
}
