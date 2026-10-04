use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use tracing::{error, warn};

use crate::chunk::Chunk;
use crate::dimension_type::DimensionType;
use crate::generator::error::PhaseFailure;
use crate::generator::phase::Phase;
use crate::generator::phase_graph::PhaseGraph;
use crate::storage::LevelStorage;

pub trait Generator: Send + Sync + Sized + 'static {
    type Terminal: Phase<Self, Output = Chunk>;
}

pub trait WorldGenerator: Send + Sync {
    fn request_chunk(&mut self, x: i32, z: i32);
    fn set_focus(&mut self, focus: &[(i32, i32)]);
    fn cancel_chunk(&mut self, x: i32, z: i32);
    fn tick(&mut self) -> Vec<(i32, i32, Result<Chunk, Arc<PhaseFailure>>)>;
    fn has_pending_work(&self) -> bool;
}

pub struct Dimension {
    pub dimension_type: DimensionType,
    generator: Box<dyn WorldGenerator>,
    chunks: HashMap<(i32, i32), Chunk>,
    modified: HashSet<(i32, i32)>,
    storage: Option<Arc<LevelStorage>>,
    unsaved: HashSet<(i32, i32)>,
    loaded: Vec<(i32, i32)>,
}

impl Dimension {
    pub fn new<G: Generator>(dimension_type: DimensionType, generator: G) -> Self {
        Self {
            dimension_type,
            generator: Box::new(PhaseGraph::new(generator)),
            chunks: HashMap::new(),
            modified: HashSet::new(),
            storage: None,
            unsaved: HashSet::new(),
            loaded: Vec::new(),
        }
    }

    pub fn with_storage(mut self, storage: Arc<LevelStorage>) -> Self {
        self.storage = Some(storage);
        self
    }

    pub fn is_persistent(&self) -> bool {
        self.storage.is_some()
    }

    pub fn unsaved_count(&self) -> usize {
        self.unsaved.len()
    }

    pub fn is_stored(&self, x: i32, z: i32) -> bool {
        self.storage.as_ref().is_some_and(|storage| storage.has_chunk(self.dimension_type, x, z))
    }

    pub fn save(&mut self) -> usize {
        let unsaved: Vec<(i32, i32)> = self.unsaved.iter().copied().collect();
        unsaved.into_iter().filter(|&position| self.save_chunk(position)).count()
    }

    fn save_chunk(&mut self, position: (i32, i32)) -> bool {
        let (Some(storage), Some(chunk)) = (&self.storage, self.chunks.get(&position)) else {
            self.unsaved.remove(&position);
            return false;
        };
        match storage.save_chunk(self.dimension_type, chunk) {
            Ok(()) => {
                self.unsaved.remove(&position);
                true
            }
            Err(err) => {
                error!("failed to save chunk ({}, {}) in {}: {err}", position.0, position.1, self.name());
                false
            }
        }
    }

    pub fn id(&self) -> i32 {
        self.dimension_type.id()
    }

    pub fn name(&self) -> &'static str {
        self.dimension_type.name()
    }

    pub fn chunk_count(&self) -> usize {
        self.chunks.len()
    }

    pub fn request_chunk(&mut self, x: i32, z: i32) {
        if self.chunks.contains_key(&(x, z)) {
            return;
        }
        if let Some(storage) = &self.storage {
            match storage.load_chunk(self.dimension_type, x, z) {
                Ok(Some(chunk)) => {
                    self.chunks.insert((x, z), chunk);
                    self.loaded.push((x, z));
                    return;
                }
                Ok(None) => {}
                Err(err) => warn!("failed to load chunk ({x}, {z}) in {}, regenerating it: {err}", self.name()),
            }
        }
        self.generator.request_chunk(x, z);
    }

    pub fn request_chunks(&mut self, positions: &[(i32, i32)]) {
        for &(x, z) in positions {
            self.request_chunk(x, z);
        }
    }

    pub fn set_focus(&mut self, focus: &[(i32, i32)]) {
        self.generator.set_focus(focus);
    }

    pub fn cancel_chunks(&mut self, positions: &[(i32, i32)]) {
        for &(x, z) in positions {
            if !self.chunks.contains_key(&(x, z)) {
                self.generator.cancel_chunk(x, z);
            }
        }
    }

    pub fn tick(&mut self) -> Vec<(i32, i32)> {
        let mut generated = std::mem::take(&mut self.loaded);
        for (x, z, result) in self.generator.tick() {
            match result {
                Ok(chunk) => {
                    self.chunks.insert((x, z), chunk);
                    if self.storage.is_some() {
                        self.unsaved.insert((x, z));
                    }
                    generated.push((x, z));
                }
                Err(failure) => error!("failed to generate chunk ({x}, {z}) in {}: {failure}", self.name()),
            }
        }
        generated
    }

    pub fn unload_chunks(&mut self, keep: impl Fn(i32, i32) -> bool) -> usize {
        let persistent = self.storage.is_some();
        let unloading: Vec<(i32, i32)> = self.chunks.keys().copied().filter(|&(x, z)| !keep(x, z) && (persistent || !self.modified.contains(&(x, z)))).collect();
        let mut unloaded = 0;
        for position in unloading {
            if self.unsaved.contains(&position) && !self.save_chunk(position) {
                continue;
            }
            self.chunks.remove(&position);
            self.modified.remove(&position);
            unloaded += 1;
        }
        unloaded
    }

    pub fn has_pending_generation(&self) -> bool {
        self.generator.has_pending_work()
    }

    pub fn get_chunk(&self, x: i32, z: i32) -> Option<&Chunk> {
        self.chunks.get(&(x, z))
    }

    pub fn get_block(&self, x: i32, y: i32, z: i32, layer: usize) -> Option<i32> {
        self.chunks.get(&(x >> 4, z >> 4)).and_then(|chunk| chunk.get_block((x & 0xF) as u8, y, (z & 0xF) as u8, layer))
    }

    pub fn set_block(&mut self, x: i32, y: i32, z: i32, layer: usize, block_id: i32) -> bool {
        let position = (x >> 4, z >> 4);
        let Some(chunk) = self.chunks.get_mut(&position) else { return false };
        let changed = chunk.set_block((x & 0xF) as u8, y, (z & 0xF) as u8, layer, block_id);
        if changed {
            self.modified.insert(position);
            if self.storage.is_some() {
                self.unsaved.insert(position);
            }
        }
        changed
    }
}
