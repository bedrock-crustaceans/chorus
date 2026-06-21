use crate::block::block_definition::BlockDefinition;
use crate::block::block_permutation::BlockPermutation;
use crate::block::component::block_components::BlockComponents;
use crate::block::r#impl::air::AIR;
use crate::block::r#impl::bedrock::BEDROCK;
use crate::block::r#impl::dirt::DIRT;
use crate::block::r#impl::grass_block::GRASS_BLOCK;
use atomicow::CowArc;
use bevy_ecs::prelude::{Commands, Resource};
use std::collections::HashMap;
use tracing::{debug, warn};

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

        registry.register_all([&AIR, &BEDROCK, &DIRT, &GRASS_BLOCK]);

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

        debug!("registered {:?}", definition.identifier);

        self.definitions.insert(definition.identifier.clone(), definition);
    }

    pub fn register_all<I, D>(&mut self, definitions: I)
    where
        I: IntoIterator<Item = D>,
        D: Into<CowArc<'static, BlockDefinition>>,
    {
        for def in definitions {
            self.register(def);
        }
    }

    pub fn get_block_id(&self, identifier: &str) -> Option<i32> {
        self.default_hash.get(identifier).map(|&h| h)
    }
}
