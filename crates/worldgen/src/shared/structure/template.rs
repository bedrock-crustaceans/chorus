use std::sync::Arc;

pub use chorus_structure_data::{FULL_CUBE, PaletteEntry, StateKind, Template};
use chorus_structure_data::{StatePool, TemplateStore};

pub struct TemplateLibrary {
    store: TemplateStore,
    pool: Arc<StatePool>,
}

impl TemplateLibrary {
    pub fn new(store: TemplateStore, pool: Arc<StatePool>) -> Self {
        Self { store, pool }
    }

    pub fn get(&self, name: &str) -> Option<Arc<Template>> {
        self.store.get(name)
    }

    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.store.names()
    }

    pub fn pool(&self) -> &Arc<StatePool> {
        &self.pool
    }

    pub fn entries(&self) -> &[PaletteEntry] {
        self.pool.entries()
    }

    pub fn entry(&self, state: u16) -> &PaletteEntry {
        self.pool.entry(state)
    }

    pub fn kind(&self, state: u16) -> StateKind {
        self.pool.kind(state)
    }

    pub fn rotated(&self, state: u16, rotation: usize) -> u16 {
        self.pool.rotated(state, rotation)
    }
}
